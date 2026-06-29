#![cfg(test)]

use super::*;
use raffle_shared::RaffleConfig;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, vec, Address, BytesN, Env, String};

fn create_token<'a>(env: &Env, admin: &Address) -> (Address, token::StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = sac.address();
    (addr.clone(), token::StellarAssetClient::new(env, &addr))
}

#[contract]
pub struct MockFactory;

#[contractimpl]
impl MockFactory {
    pub fn record_volume(_env: Env, _token: Address, _amount: i128) {}
    pub fn track_participant(_env: Env, _participant: Address) {}
}

#[test]
fn non_winner_cannot_claim() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let buyer = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_addr, token_mint) = create_token(&env, &token_admin);
    token_mint.mint(&creator, &1_000_000);
    token_mint.mint(&buyer, &1_000_000);

    let config = RaffleConfig {
        description: String::from_str(&env, "test raffle"),
        end_time: 2_000,
        no_deadline: false,
        max_tickets: 2,
        max_tickets_per_tx: 2,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr.clone(),
        prize_amount: MIN_TICKET_PRICE * 10,
        prizes: vec![&env, 10000u32],
        randomness_source: RandomnessSource::Internal,
        oracle_address: None,
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(&env, &[1u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();
    client.buy_tickets(&buyer, &1);
    env.ledger().set_timestamp(2_000);
    env.ledger().set_timestamp(2_000);
    client.finalize_raffle();

    let raffle = client.get_raffle();
    assert_eq!(raffle.winners.len(), 1);
    assert!(raffle.winners.get(0).unwrap() != attacker);

    env.ledger().set_timestamp(2_000 + DEFAULT_CLAIM_LOCKUP_SECONDS + 1);

    let result = client.try_claim_prize(&attacker, &0u32);
    assert_eq!(result, Err(Ok(Error::NotWinner)));
}

#[test]
fn buy_tickets_rejects_quantity_above_per_tx_cap() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let buyer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_addr, token_mint) = create_token(&env, &token_admin);
    token_mint.mint(&creator, &1_000_000);
    token_mint.mint(&buyer, &1_000_000);

    let config = RaffleConfig {
        description: String::from_str(&env, "Per-tx cap"),
        end_time: 0,
        no_deadline: true,
        max_tickets: 100,
        max_tickets_per_tx: 5,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr.clone(),
        prize_amount: MIN_TICKET_PRICE * 100,
        prizes: vec![&env, 10000u32],
        randomness_source: RandomnessSource::Internal,
        oracle_address: None,
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(&env, &[5u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();

    assert_eq!(
        client.try_buy_tickets(&buyer, &6),
        Err(Ok(Error::ExceedsMaxTicketsPerTx))
    );
    assert_eq!(client.buy_tickets(&buyer, &5), 5);
}

fn setup_active_raffle(
    env: &Env,
) -> (
    ContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
    token::StellarAssetClient<'_>,
) {
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let buyer = Address::generate(env);

    let token_admin = Address::generate(env);
    let (token_addr, token_mint) = create_token(env, &token_admin);
    token_mint.mint(&creator, &1_000_000);
    token_mint.mint(&buyer, &1_000_000);

    let config = RaffleConfig {
        description: String::from_str(env, "ticket sales pause"),
        end_time: 0,
        no_deadline: true,
        max_tickets: 100,
        max_tickets_per_tx: 10,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr,
        prize_amount: MIN_TICKET_PRICE * 100,
        prizes: vec![env, 10000u32],
        randomness_source: RandomnessSource::Internal,
        oracle_address: None,
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(env, &[7u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();

    (client, admin, creator, buyer, factory, token_mint)
}

#[test]
fn pause_resume_ticket_sales_controls_buy_tickets() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let (client, _admin, creator, buyer, _factory, _token_mint) = setup_active_raffle(&env);

    assert_eq!(client.get_raffle().status, RaffleStatus::Active);
    assert!(!client.is_ticket_sales_paused());

    client.pause_ticket_sales(&creator);
    assert!(client.is_ticket_sales_paused());
    assert_eq!(client.get_raffle().status, RaffleStatus::Active);
    assert_eq!(
        client.try_buy_tickets(&buyer, &1),
        Err(Ok(Error::ContractPaused))
    );

    client.resume_ticket_sales(&creator);
    assert!(!client.is_ticket_sales_paused());
    assert_eq!(client.get_raffle().status, RaffleStatus::Active);
    assert_eq!(client.buy_tickets(&buyer, &1), 1);
}

#[test]
fn admin_can_pause_and_resume_ticket_sales() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let (client, admin, _creator, buyer, _factory, _token_mint) = setup_active_raffle(&env);

    client.pause_ticket_sales(&admin);
    assert!(client.is_ticket_sales_paused());
    assert_eq!(
        client.try_buy_tickets(&buyer, &1),
        Err(Ok(Error::ContractPaused))
    );

    client.resume_ticket_sales(&admin);
    assert!(!client.is_ticket_sales_paused());
    assert_eq!(client.buy_tickets(&buyer, &1), 1);
}

#[test]
fn test_wipe_storage_removes_all_keys() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let buyer_a = Address::generate(&env);
    let buyer_b = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_addr, token_mint) = create_token(&env, &token_admin);
    token_mint.mint(&creator, &1_000_000);
    token_mint.mint(&buyer_a, &1_000_000);
    token_mint.mint(&buyer_b, &1_000_000);

    let config = RaffleConfig {
        description: String::from_str(&env, "wipe test"),
        end_time: 0,
        no_deadline: true,
        max_tickets: 10,
        max_tickets_per_tx: 10,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr,
        prize_amount: MIN_TICKET_PRICE * 10,
        prizes: vec![&env, 10000u32],
        randomness_source: RandomnessSource::Internal,
        oracle_address: None,
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(&env, &[9u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();
    client.buy_tickets(&buyer_a, &3);
    client.buy_tickets(&buyer_b, &2);

    client.cancel_raffle(&CancelReason::AdminCancelled);

    assert_eq!(client.get_raffle().status, RaffleStatus::Cancelled);

    client.wipe_storage();

    env.as_contract(&contract_id, || {
        for i in 1..=5 {
            assert!(!env.storage().persistent().has(&DataKey::Ticket(i)));
            assert!(!env.storage().persistent().has(&DataKey::TicketRefunded(i)));
            assert!(!env.storage().persistent().has(&DataKey::CommitEntry(i)));
        }
        assert!(!env.storage().persistent().has(&DataKey::TicketCount(buyer_a.clone())));
        assert!(!env.storage().persistent().has(&DataKey::TicketCount(buyer_b.clone())));
        assert!(!env.storage().persistent().has(&DataKey::TicketBuyers));
        assert!(!env.storage().instance().has(&DataKey::Raffle));
        assert!(!env.storage().instance().has(&DataKey::Factory));
        assert!(!env.storage().instance().has(&DataKey::Admin));
        assert!(!env.storage().instance().has(&DataKey::Paused));
        assert!(!env.storage().instance().has(&DataKey::ReentrancyGuard));
        assert!(!env.storage().instance().has(&DataKey::AccumulatedFees));
        assert!(!env.storage().instance().has(&DataKey::RandomnessRequested));
        assert!(!env.storage().instance().has(&DataKey::RandomnessRequestLedger));
        assert!(!env.storage().instance().has(&DataKey::RandomnessRequestId));
        assert!(!env.storage().instance().has(&DataKey::DrawingLock));
        assert!(!env.storage().instance().has(&DataKey::FinishTime));
        assert!(!env.storage().persistent().has(&DataKey::RandomnessSeed));
        assert!(!env.storage().persistent().has(&DataKey::Admin));
    });
}

#[test]
fn emergency_withdraw_no_deadline_drawing_respects_delay() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_addr, token_mint) = create_token(&env, &token_admin);
    token_mint.mint(&creator, &10_000_000);

    let config = RaffleConfig {
        description: String::from_str(&env, "no-deadline drawing"),
        end_time: 0,
        no_deadline: true,
        max_tickets: 5,
        max_tickets_per_tx: 5,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr.clone(),
        prize_amount: MIN_TICKET_PRICE * 5,
        prizes: vec![&env, 10000u32],
        randomness_source: RandomnessSource::External,
        oracle_address: Some(oracle.clone()),
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(&env, &[3u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();
    client.buy_tickets(&creator, &5);

    let raffle = client.get_raffle();
    assert_eq!(raffle.status, RaffleStatus::Drawing);
    assert!(raffle.no_deadline);

    let too_early = client.try_emergency_withdraw(&creator);
    assert_eq!(too_early.err(), Some(Ok(Error::EmergencyTooEarly)));

    let ledgers_for_delay = (EMERGENCY_WITHDRAW_DELAY_SECONDS / 5) as u32 + 1;
    env.ledger().with_mut(|l| { l.sequence_number += ledgers_for_delay; });

    client.emergency_withdraw(&creator);

    let after = client.get_raffle();
    assert_eq!(after.status, RaffleStatus::Cancelled);
    assert!(!after.prize_deposited);
}

#[test]
fn emergency_withdraw_deadline_drawing_respects_end_time_delay() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_addr, token_mint) = create_token(&env, &token_admin);
    token_mint.mint(&creator, &10_000_000);

    let end_time = 5_000u64;
    let config = RaffleConfig {
        description: String::from_str(&env, "deadline drawing"),
        end_time,
        no_deadline: false,
        max_tickets: 5,
        max_tickets_per_tx: 5,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr.clone(),
        prize_amount: MIN_TICKET_PRICE * 5,
        prizes: vec![&env, 10000u32],
        randomness_source: RandomnessSource::External,
        oracle_address: Some(oracle.clone()),
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(&env, &[4u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();
    client.buy_tickets(&creator, &3);
    env.ledger().set_timestamp(end_time);
    client.finalize_raffle();

    let raffle = client.get_raffle();
    assert_eq!(raffle.status, RaffleStatus::Drawing);
    assert!(!raffle.no_deadline);

    let too_early = client.try_emergency_withdraw(&creator);
    assert_eq!(too_early.err(), Some(Ok(Error::EmergencyTooEarly)));

    env.ledger().set_timestamp(end_time + EMERGENCY_WITHDRAW_DELAY_SECONDS + 1);
    client.emergency_withdraw(&creator);

    let after = client.get_raffle();
    assert_eq!(after.status, RaffleStatus::Cancelled);
}

fn setup_external_drawing_raffle(
    env: &Env,
) -> (Address, ContractClient<'_>, Address, Address, Address, u64) {
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);

    let factory = env.register(MockFactory, ());
    let admin = Address::generate(env);
    let creator = Address::generate(env);
    let oracle = Address::generate(env);

    let token_admin = Address::generate(env);
    let (token_addr, token_mint) = create_token(env, &token_admin);
    token_mint.mint(&creator, &10_000_000);

    let config = RaffleConfig {
        description: String::from_str(env, "vrf proof test"),
        end_time: 0,
        no_deadline: true,
        max_tickets: 3,
        max_tickets_per_tx: 3,
        min_tickets: 1,
        allow_multiple: true,
        ticket_price: MIN_TICKET_PRICE,
        payment_token: token_addr,
        prize_amount: MIN_TICKET_PRICE * 3,
        prizes: vec![env, 10000u32],
        randomness_source: RandomnessSource::External,
        oracle_address: Some(oracle.clone()),
        protocol_fee_bp: 0,
        treasury_address: None,
        swap_router: None,
        tikka_token: None,
        metadata_hash: BytesN::from_array(env, &[5u8; 32]),
        claim_lockup_seconds: 0,
        swap_deadline_seconds: 0,
    };

    client.init(&factory, &admin, &creator, &config);
    client.deposit_prize();
    client.buy_tickets(&creator, &3);

    let request_id: u64 = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::RandomnessRequestId).unwrap()
    });

    (contract_id, client, creator, oracle, admin, request_id)
}

#[test]
fn vrf_proof_valid_for_target_raffle_only() {
    use ed25519_dalek::{Signer, SigningKey};

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let signing_key = SigningKey::from_bytes(&[9u8; 32]);
    let public_key = BytesN::from_array(&env, &signing_key.verifying_key().to_bytes());

    let (contract_a, client_a, _creator_a, _oracle_a, _admin_a, request_id_a) =
        setup_external_drawing_raffle(&env);
    let (_contract_b, client_b, _creator_b, _oracle_b, _admin_b, request_id_b) =
        setup_external_drawing_raffle(&env);

    let random_seed = 0xDEAD_BEEF_u64;

    let message_a = env.as_contract(&contract_a, || {
        randomness::build_vrf_proof_message(&env, request_id_a, random_seed)
    });
    let mut msg_a = [0u8; 256];
    let msg_len = message_a.len() as usize;
    for (idx, byte) in message_a.iter().enumerate() {
        msg_a[idx] = byte;
    }
    let proof_a = BytesN::from_array(&env, &signing_key.sign(&msg_a[..msg_len]).to_bytes());

    client_a.provide_randomness(&random_seed, &public_key, &proof_a, &request_id_a);
    assert_eq!(client_a.get_raffle().status, RaffleStatus::Finalized);

    let replay = client_b.try_provide_randomness(&random_seed, &public_key, &proof_a, &request_id_b);
    assert!(replay.is_err());
}
