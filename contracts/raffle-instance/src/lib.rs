#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, String, Vec,
};

pub mod admin;
pub mod claim;
pub mod draw;
pub mod events;
pub mod helpers;
pub mod init;
pub mod randomness;
pub mod tickets;
pub mod views;

pub use raffle_shared::{
    CancelReason, FailureReason, FairnessData, RaffleConfig, RaffleStatus, RandomnessSource,
    RandomnessType, Ticket,
};

pub(crate) use helpers::*;

const ORACLE_TIMEOUT_LEDGERS: u32 = 200;
pub const MAX_DESCRIPTION_LENGTH: u32 = 1000;
pub const MAX_TICKETS_LIMIT: u32 = 100_000;
pub const MAX_PRIZES: u32 = 100;
pub const MIN_TICKET_PRICE: i128 = 10_000;
pub const MAX_PRIZE_AMOUNT: i128 = 1_000_000_000_000_000_000_000;
pub const DEFAULT_CLAIM_LOCKUP_SECONDS: u64 = 3_600;
pub const MAX_CLAIM_LOCKUP_SECONDS: u64 = 604_800;
pub const DEFAULT_SWAP_DEADLINE_SECONDS: u64 = 300;
pub const MAX_SWAP_DEADLINE_SECONDS: u64 = 3_600;
pub const EMERGENCY_WITHDRAW_DELAY_SECONDS: u64 = 90 * 24 * 3600;
pub const MAX_PROTOCOL_FEE_BP: u32 = 2_000;

#[contract]
pub struct Contract;

#[contracttype]
#[derive(Clone)]
pub struct Raffle {
    pub creator: Address,
    pub description: String,
    pub end_time: u64,
    pub no_deadline: bool,
    pub max_tickets: u32,
    pub max_tickets_per_tx: u32,
    pub min_tickets: u32,
    pub allow_multiple: bool,
    pub ticket_price: i128,
    pub payment_token: Address,
    pub prize_amount: i128,
    pub prizes: Vec<u32>,
    pub tickets_sold: u32,
    pub status: RaffleStatus,
    pub prize_deposited: bool,
    pub winners: Vec<Address>,
    pub claimed_winners: Vec<bool>,
    pub randomness_source: RandomnessSource,
    pub oracle_address: Option<Address>,
    pub protocol_fee_bp: u32,
    pub treasury_address: Option<Address>,
    pub swap_router: Option<Address>,
    pub tikka_token: Option<Address>,
    pub finalized_at: Option<u64>,
    pub claim_lockup_seconds: u64,
    pub swap_deadline_seconds: u64,
    pub ticket_sales_paused: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct FairnessMetadata {
    pub seed: u64,
    pub randomness_source: RandomnessSource,
    pub winning_ticket_indices: Vec<u32>,
    pub draw_timestamp: u64,
    pub draw_sequence: u32,
}

#[soroban_sdk::contracttype]
#[derive(Clone)]
pub enum DataKey {
    Raffle,
    TicketCount(Address),
    Ticket(u32),
    TicketRefunded(u32),
    Factory,
    ReentrancyGuard,
    Paused,
    Admin,
    RandomnessSeed,
    RandomnessRequested,
    RandomnessRequestLedger,
    RandomnessRequestId,
    FinishTime,
    AccumulatedFees,
    CommitEntry(u32),
    DrawingLock,
    TicketBuyers,
}

#[contracttype]
#[derive(Clone)]
pub struct CommitRevealEntry {
    pub committer: Address,
    pub hash: BytesN<32>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    RaffleNotFound = 1,
    RaffleInactive = 2,
    TicketsSoldOut = 3,
    InsufficientFunds = 4,
    NotAuthorized = 5,
    OracleNotSet = 6,
    RandomnessAlreadyRequested = 7,
    NoRandomnessRequest = 8,
    FallbackTooEarly = 9,
    PrizeNotDeposited = 11,
    PrizeAlreadyClaimed = 12,
    PrizeAlreadyDeposited = 13,
    NotWinner = 14,
    ClaimTooEarly = 15,
    InvalidParameters = 21,
    InvalidQuantity = 22,
    InvalidStatus = 23,
    ContractPaused = 24,
    InvalidStateTransition = 25,
    RaffleExpired = 26,
    InsufficientTickets = 31,
    MultipleTicketsNotAllowed = 32,
    NoTicketsSold = 33,
    TicketNotFound = 34,
    RaffleEnded = 35,
    ArithmeticOverflow = 41,
    AlreadyInitialized = 42,
    NotInitialized = 43,
    Reentrancy = 44,
    TokenTransferFailed = 45,
    NoActiveTickets = 46,
    DeadlinePassed = 47,
    SlippageExceeded = 48,
    InvalidIndex = 49,
    MorePrizesThanTickets = 50,
    ZeroPrize = 51,
    InvalidTokenAddress = 52,
    TooManyPrizes = 53,
    EmergencyTooEarly = 54,
    InvalidTicketRange = 55,
    InsufficientAccumulatedFees = 56,
    PrizeConfigurationLocked = 57,
    ExceedsMaxTicketsPerTx = 58,
    DrawingAlreadyInProgress = 59,
    InvalidStatusForDrawingTransition = 60,
    DrawingAlreadyComplete = 61,
    InvalidEndTime = 62,
    InvalidAdminAddress = 63,
}

#[contractimpl]
impl Contract {
    pub fn init(env: Env, factory: Address, admin: Address, creator: Address, config: RaffleConfig) -> Result<(), Error> {
        self::init::init(env, factory, admin, creator, config)
    }

    pub fn deposit_prize(env: Env) -> Result<(), Error> {
        self::init::deposit_prize(env)
    }

    pub fn buy_tickets(env: Env, buyer: Address, quantity: u32) -> Result<u32, Error> {
        self::tickets::buy_tickets(env, buyer, quantity)
    }

    pub fn submit_commit(env: Env, ticket_id: u32, hash: BytesN<32>) -> Result<(), Error> {
        self::tickets::submit_commit(env, ticket_id, hash)
    }

    pub fn finalize_raffle(env: Env) -> Result<(), Error> {
        self::draw::finalize_raffle(env)
    }

    pub fn provide_randomness(env: Env, random_seed: u64, public_key: BytesN<32>, proof: BytesN<64>, request_id: u64) -> Result<Address, Error> {
        self::draw::provide_randomness(env, random_seed, public_key, proof, request_id)
    }

    pub fn trigger_randomness_fallback(env: Env, caller: Address, do_refund: bool) -> Result<(), Error> {
        self::draw::trigger_randomness_fallback(env, caller, do_refund)
    }

    pub fn claim_prize(env: Env, winner: Address, tier_index: u32) -> Result<i128, Error> {
        self::claim::claim_prize(env, winner, tier_index)
    }

    pub fn refund_prize(env: Env) -> Result<(), Error> {
        self::claim::refund_prize(env)
    }

    pub fn refund_ticket(env: Env, ticket_id: u32) -> Result<i128, Error> {
        self::claim::refund_ticket(env, ticket_id)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        self::admin::set_admin(env, new_admin)
    }

    pub fn update_oracle_address(env: Env, new_oracle: Address) -> Result<(), Error> {
        self::admin::update_oracle_address(env, new_oracle)
    }

    pub fn set_protocol_fee_bp(env: Env, new_fee_bp: u32) -> Result<(), Error> {
        self::admin::set_protocol_fee_bp(env, new_fee_bp)
    }

    pub fn set_swap_deadline(env: Env, new_deadline_seconds: u64) -> Result<(), Error> {
        self::admin::set_swap_deadline(env, new_deadline_seconds)
    }

    pub fn cancel_raffle(env: Env, reason: CancelReason) -> Result<(), Error> {
        self::admin::cancel_raffle(env, reason)
    }

    pub fn pause(env: Env) -> Result<(), Error> {
        self::admin::pause(env)
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        self::admin::unpause(env)
    }

    pub fn pause_ticket_sales(env: Env, caller: Address) -> Result<(), Error> {
        self::admin::pause_ticket_sales(env, caller)
    }

    pub fn resume_ticket_sales(env: Env, caller: Address) -> Result<(), Error> {
        self::admin::resume_ticket_sales(env, caller)
    }

    pub fn withdraw_fees(env: Env, recipient: Address, amount: i128) -> Result<(), Error> {
        self::admin::withdraw_fees(env, recipient, amount)
    }

    pub fn rescue_tokens(env: Env, token: Address, recipient: Address, amount: i128) -> Result<(), Error> {
        self::admin::rescue_tokens(env, token, recipient, amount)
    }

    pub fn wipe_storage(env: Env) -> Result<(), Error> {
        self::admin::wipe_storage(env)
    }

    pub fn emergency_withdraw(env: Env, caller: Address) -> Result<(), Error> {
        self::admin::emergency_withdraw(env, caller)
    }

    pub fn get_raffle(env: Env) -> Result<Raffle, Error> {
        self::views::get_raffle(env)
    }

    pub fn get_fairness_data(env: Env) -> Result<FairnessData, Error> {
        self::views::get_fairness_data(env)
    }

    pub fn is_paused(env: Env) -> bool {
        self::views::is_paused(env)
    }

    pub fn is_ticket_sales_paused(env: Env) -> bool {
        self::views::is_ticket_sales_paused(env)
    }

    pub fn get_accumulated_fees(env: Env) -> i128 {
        self::views::get_accumulated_fees(env)
    }
}

#[cfg(test)]
mod test;
