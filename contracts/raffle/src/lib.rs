#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, xdr::ToXdr, Address, Bytes, Env, String, Vec,
};

mod instance;
use instance::{RaffleConfig, RandomnessSource};

#[contract]
pub struct RaffleFactory;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    RaffleInstances,
    InstanceWasmHash,
}

#[contractimpl]
impl RaffleFactory {
    pub fn init(env: Env, admin: Address, wasm_hash: Bytes) {
        if env.storage().persistent().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::InstanceWasmHash, &wasm_hash);
        env.storage()
            .persistent()
            .set(&DataKey::RaffleInstances, &Vec::<Address>::new(&env));
    }

    pub fn create_raffle(
        env: Env,
        creator: Address,
        description: String,
        end_time: u64,
        max_tickets: u32,
        allow_multiple: bool,
        ticket_price: i128,
        payment_token: Address,
        prize_amount: i128,
        randomness_source: RandomnessSource,
        oracle_address: Option<Address>,
    ) -> Address {
        creator.require_auth();

        let _wasm_hash: Bytes = env
            .storage()
            .persistent()
            .get(&DataKey::InstanceWasmHash)
            .unwrap();

        let mut _salt_src = Vec::new(&env);
        _salt_src.push_back(creator.clone());
        let _salt = env.crypto().sha256(&creator.clone().to_xdr(&env));

        // Deployment logic placeholder
        // let client = instance::ContractClient::new(&env, &instance_address);
        // let config = RaffleConfig { ... };
        // client.init(&env.current_contract_address(), &creator, &config);

        let mut instances: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::RaffleInstances)
            .unwrap();

        // Use parameters to avoid warnings
        let _ = RaffleConfig {
            description,
            end_time,
            max_tickets,
            allow_multiple,
            ticket_price,
            payment_token,
            prize_amount,
            randomness_source,
            oracle_address,
        };

        instances.push_back(creator.clone());
        env.storage()
            .persistent()
            .set(&DataKey::RaffleInstances, &instances);

        creator
    }

    pub fn get_raffles(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::RaffleInstances)
            .unwrap_or_else(|| Vec::new(&env))
    }
}
