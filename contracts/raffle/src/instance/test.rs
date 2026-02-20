#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, String, Symbol,
};

/// HELPER: Standardized environment setup
fn setup_raffle_env(
    env: &Env,
) -> (
    ContractClient<'_>,
    Address,
    Address,
    token::StellarAssetClient<'_>,
    Address,
) {
    let creator = Address::generate(env);
    let buyer = Address::generate(env);
    let admin = Address::generate(env);
    let factory = Address::generate(env);

    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    let admin_client = token::StellarAssetClient::new(env, &token_id);

    admin_client.mint(&creator, &1_000i128);
    admin_client.mint(&buyer, &1_000i128);

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);

    client.init(
        &factory,
        &creator,
        &String::from_str(env, "Audit Raffle"),
        &0,
        &10,
        &false,
        &10i128,
        &token_id,
        &100i128,
    );

    (client, creator, buyer, admin_client, factory)
}

// --- 1. FUNCTIONAL FLOW TESTS ---

#[test]
fn test_basic_raffle_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, creator, buyer, admin_client, _) = setup_raffle_env(&env);
    let token_client = token::Client::new(&env, &admin_client.address);

    client.deposit_prize();
    client.buy_ticket(&buyer);

    let winner = client.finalize_raffle(&String::from_str(&env, "prng"));
    let _claimed_amount = client.claim_prize(&winner);

    assert_eq!(token_client.balance(&winner), 1_090i128);
    assert_eq!(token_client.balance(&creator), 900i128);
}

// --- 2. RANDOMNESS SOURCE TESTS ---

#[test]
fn test_randomness_source_prng() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, buyer, _, _) = setup_raffle_env(&env);

    client.deposit_prize();
    client.buy_ticket(&buyer);

    let source = String::from_str(&env, "prng");
    let winner = client.finalize_raffle(&source);

    assert_eq!(winner, buyer);
}

#[test]
fn test_randomness_source_oracle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, buyer, _, _) = setup_raffle_env(&env);

    client.deposit_prize();
    client.buy_ticket(&buyer);

    let source = String::from_str(&env, "oracle");
    let winner = client.finalize_raffle(&source);

    assert_eq!(winner, buyer);
}

// --- 3. EVENT AUDIT & STATE VALIDATION ---

#[test]
fn test_raffle_finalized_event_audit() {
    let env = Env::default();
    env.mock_all_auths();

    let expected_timestamp = 123456789;
    env.ledger().with_mut(|l| {
        l.timestamp = expected_timestamp;
    });

    let (client, _, buyer_1, admin_client, _) = setup_raffle_env(&env);

    let buyer_2 = Address::generate(&env);
    admin_client.mint(&buyer_2, &1_000i128);

    client.deposit_prize();
    client.buy_ticket(&buyer_1);
    client.buy_ticket(&buyer_2);

    let _winner = client.finalize_raffle(&String::from_str(&env, "oracle"));

    let last_event = env.events().all().last().expect("No event emitted");

    let topic_0: Symbol = last_event.1.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "RaffleFinalized"));
}

#[test]
fn test_single_ticket_purchase_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, buyer, _, _) = setup_raffle_env(&env);

    client.deposit_prize();

    let _ = env.events().all();

    client.buy_ticket(&buyer);

    let events = env.events().all();
    let last_event = events.last().expect("No events");
    let topic_0: Symbol = last_event.1.get(0).unwrap().into_val(&env);
    assert_eq!(topic_0, Symbol::new(&env, "TicketPurchased"));
}
