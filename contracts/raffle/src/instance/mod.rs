// Instance submodule
use core::cmp::min;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env,
    String, Vec,
};

#[contract]
pub struct Contract;

#[derive(Clone)]
#[contracttype]
pub struct Raffle {
    pub creator: Address,
    pub description: String,
    pub end_time: u64,
    pub max_tickets: u32,
    pub allow_multiple: bool,
    pub ticket_price: i128,
    pub payment_token: Address,
    pub prize_amount: i128,
    pub tickets_sold: u32,
    pub is_active: bool,
    pub prize_deposited: bool,
    pub prize_claimed: bool,
    pub winner: Option<Address>,
}

#[derive(Clone, PartialEq, Eq)]
#[contracttype]
pub enum RaffleStatus {
    Proposed,
    Active,
    Drawing,
    Finalized,
    Claimed,
}

#[derive(Clone)]
#[contracttype]
pub struct RaffleStats {
    pub tickets_sold: u32,
    pub max_tickets: u32,
    pub tickets_remaining: u32,
    pub total_revenue: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct Ticket {
    pub id: u32,
    pub buyer: Address,
    pub purchase_time: u64,
    pub ticket_number: u32,
}

// --- Events ---

#[contractevent(topics = ["PrizeClaimed"])]
#[derive(Clone)]
pub struct PrizeClaimed {
    pub winner: Address,
    pub gross_amount: i128,
    pub net_amount: i128,
    pub platform_fee: i128,
    pub claimed_at: u64,
}

#[contractevent(topics = ["RaffleInitialized"])]
#[derive(Clone)]
pub struct RaffleInitialized {
    pub creator: Address,
    pub end_time: u64,
    pub max_tickets: u32,
    pub ticket_price: i128,
    pub payment_token: Address,
    pub description: String,
}

#[contractevent(topics = ["RaffleFinalized"])]
#[derive(Clone, Debug)]
pub struct RaffleFinalized {
    pub winner: Address,
    pub winning_ticket_id: u32,
    pub total_tickets_sold: u32,
    pub randomness_source: String,
    pub finalized_at: u64,
}

#[contractevent(topics = ["TicketPurchased"])]
#[derive(Clone)]
pub struct TicketPurchased {
    pub buyer: Address,
    pub ticket_ids: Vec<u32>,
    pub quantity: u32,
    pub total_paid: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Raffle,
    Tickets,
    TicketCount(Address),
    Ticket(u32),
    NextTicketId,
    Factory,
}

// --- Error Types ---

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    RaffleNotFound = 1,
    RaffleInactive = 2,
    TicketsSoldOut = 3,
    InsufficientPayment = 4,
    NotAuthorized = 5,
    PrizeNotDeposited = 6,
    PrizeAlreadyClaimed = 7,
    InvalidParameters = 8,
    ContractPaused = 9,
    InsufficientTickets = 10,
    RaffleEnded = 11,
    RaffleStillRunning = 12,
    NoTicketsSold = 13,
    MultipleTicketsNotAllowed = 14,
    PrizeAlreadyDeposited = 15,
    NotWinner = 16,
    ArithmeticOverflow = 17,
    AlreadyInitialized = 18,
    NotInitialized = 19,
}

const MAX_PAGE_LIMIT: u32 = 100;

fn read_raffle(env: &Env) -> Result<Raffle, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Raffle)
        .ok_or(Error::NotInitialized)
}

fn write_raffle(env: &Env, raffle: &Raffle) {
    env.storage().persistent().set(&DataKey::Raffle, raffle);
}

fn read_tickets(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Tickets)
        .unwrap_or_else(|| Vec::new(env))
}

fn write_tickets(env: &Env, tickets: &Vec<Address>) {
    env.storage().persistent().set(&DataKey::Tickets, tickets);
}

fn read_ticket_count(env: &Env, buyer: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::TicketCount(buyer.clone()))
        .unwrap_or(0)
}

fn write_ticket_count(env: &Env, buyer: &Address, count: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::TicketCount(buyer.clone()), &count);
}

fn next_ticket_id(env: &Env) -> u32 {
    let current = env
        .storage()
        .persistent()
        .get(&DataKey::NextTicketId)
        .unwrap_or(0u32);
    let next = current + 1;
    env.storage()
        .persistent()
        .set(&DataKey::NextTicketId, &next);
    next
}

fn write_ticket(env: &Env, ticket: &Ticket) {
    env.storage()
        .persistent()
        .set(&DataKey::Ticket(ticket.id), ticket);
}

#[contractimpl]
impl Contract {
    pub fn init(
        env: Env,
        factory: Address,
        creator: Address,
        description: String,
        end_time: u64,
        max_tickets: u32,
        allow_multiple: bool,
        ticket_price: i128,
        payment_token: Address,
        prize_amount: i128,
    ) -> Result<(), Error> {
        if env.storage().persistent().has(&DataKey::Raffle) {
            return Err(Error::AlreadyInitialized);
        }

        let now = env.ledger().timestamp();
        if end_time < now && end_time != 0 {
            return Err(Error::InvalidParameters);
        }
        if max_tickets == 0 {
            return Err(Error::InvalidParameters);
        }
        if ticket_price <= 0 {
            return Err(Error::InvalidParameters);
        }
        if prize_amount <= 0 {
            return Err(Error::InvalidParameters);
        }

        let raffle = Raffle {
            creator: creator.clone(),
            description: description.clone(),
            end_time,
            max_tickets,
            allow_multiple,
            ticket_price,
            payment_token: payment_token.clone(),
            prize_amount,
            tickets_sold: 0,
            is_active: true,
            prize_deposited: false,
            prize_claimed: false,
            winner: None,
        };
        write_raffle(&env, &raffle);
        env.storage().persistent().set(&DataKey::Factory, &factory);

        RaffleInitialized {
            creator,
            end_time,
            max_tickets,
            ticket_price,
            payment_token,
            description,
        }
        .publish(&env);

        Ok(())
    }

    pub fn deposit_prize(env: Env) -> Result<(), Error> {
        let mut raffle = read_raffle(&env)?;
        raffle.creator.require_auth();
        if !raffle.is_active {
            return Err(Error::RaffleInactive);
        }
        if raffle.prize_deposited {
            return Err(Error::PrizeAlreadyDeposited);
        }

        let token_client = token::Client::new(&env, &raffle.payment_token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&raffle.creator, &contract_address, &raffle.prize_amount);

        raffle.prize_deposited = true;
        write_raffle(&env, &raffle);
        Ok(())
    }

    pub fn buy_ticket(env: Env, buyer: Address) -> Result<u32, Error> {
        buyer.require_auth();
        let mut raffle = read_raffle(&env)?;
        if !raffle.is_active {
            return Err(Error::RaffleInactive);
        }
        if raffle.end_time != 0 && env.ledger().timestamp() > raffle.end_time {
            return Err(Error::RaffleEnded);
        }
        if raffle.tickets_sold >= raffle.max_tickets {
            return Err(Error::TicketsSoldOut);
        }

        let current_count = read_ticket_count(&env, &buyer);
        if !raffle.allow_multiple && current_count > 0 {
            return Err(Error::MultipleTicketsNotAllowed);
        }

        let token_client = token::Client::new(&env, &raffle.payment_token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&buyer, &contract_address, &raffle.ticket_price);

        let ticket_id = next_ticket_id(&env);
        let timestamp = env.ledger().timestamp();

        let ticket = Ticket {
            id: ticket_id,
            buyer: buyer.clone(),
            purchase_time: timestamp,
            ticket_number: raffle.tickets_sold + 1,
        };
        write_ticket(&env, &ticket);

        let mut tickets = read_tickets(&env);
        tickets.push_back(buyer.clone());
        write_tickets(&env, &tickets);

        raffle.tickets_sold += 1;
        write_ticket_count(&env, &buyer, current_count + 1);
        write_raffle(&env, &raffle);

        let mut ticket_ids = Vec::new(&env);
        ticket_ids.push_back(ticket_id);

        TicketPurchased {
            buyer,
            ticket_ids,
            quantity: 1u32,
            total_paid: raffle.ticket_price,
            timestamp,
        }
        .publish(&env);

        Ok(raffle.tickets_sold)
    }

    pub fn finalize_raffle(env: Env, source: String) -> Result<Address, Error> {
        let mut raffle = read_raffle(&env)?;
        raffle.creator.require_auth();
        if !raffle.is_active {
            return Err(Error::RaffleInactive);
        }
        if raffle.end_time != 0 && env.ledger().timestamp() < raffle.end_time {
            return Err(Error::RaffleStillRunning);
        }
        if raffle.tickets_sold == 0 {
            return Err(Error::NoTicketsSold);
        }

        let tickets = read_tickets(&env);
        let seed = env.ledger().timestamp() + env.ledger().sequence() as u64;
        let winner_index = (seed % tickets.len() as u64) as u32;
        let winner = tickets.get(winner_index).unwrap();

        raffle.is_active = false;
        raffle.winner = Some(winner.clone());
        write_raffle(&env, &raffle);

        RaffleFinalized {
            winner: winner.clone(),
            winning_ticket_id: winner_index,
            total_tickets_sold: raffle.tickets_sold,
            randomness_source: source,
            finalized_at: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(winner)
    }

    pub fn claim_prize(env: Env, winner: Address) -> Result<i128, Error> {
        winner.require_auth();
        let mut raffle = read_raffle(&env)?;
        if raffle.winner != Some(winner.clone()) {
            return Err(Error::NotWinner);
        }
        if !raffle.prize_deposited {
            return Err(Error::PrizeNotDeposited);
        }
        if raffle.prize_claimed {
            return Err(Error::PrizeAlreadyClaimed);
        }

        let net_amount = raffle.prize_amount;
        let claimed_at = env.ledger().timestamp();

        let token_client = token::Client::new(&env, &raffle.payment_token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &winner, &net_amount);

        PrizeClaimed {
            winner: winner.clone(),
            gross_amount: raffle.prize_amount,
            net_amount,
            platform_fee: 0,
            claimed_at,
        }
        .publish(&env);

        raffle.prize_claimed = true;
        write_raffle(&env, &raffle);
        Ok(net_amount)
    }

    pub fn get_raffle(env: Env) -> Result<Raffle, Error> {
        read_raffle(&env)
    }
}

#[cfg(test)]
mod test;
