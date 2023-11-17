#![no_main]

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
struct TokenState {
    ticker: String,
    owner: AccountId,
    total_supply: u128,
    balances: std::collections::HashMap<AccountId, u128>,
}

pub struct TokenContract;

#[multivm_sdk_macros::contract]
impl TokenContract {
    pub fn init(input: (String, u128)) {
        let ticker = input.0;
        let initial_supply = input.1;

        let mut state = TokenState {
            ticker,
            owner: env::caller().clone(),
            total_supply: initial_supply,
            balances: std::collections::HashMap::new(),
        };

        state.balances.insert(env::caller(), initial_supply);

        env::set_storage(String::from("root"), state);
        env::commit(())
    }

    pub fn symbol() {
        let state: TokenState = env::get_storage(String::from("root")).unwrap();
        env::commit(state.ticker);
    }

    pub fn balance_of(address: AccountId) {
        let state: TokenState = env::get_storage(String::from("root")).unwrap();
        let balance = state.balances.get(&address).unwrap_or(&0);

        env::commit(balance);
    }

    pub fn mint(amount: u128) {
        let mut state: TokenState = env::get_storage(String::from("root")).unwrap();

        if state.owner != env::caller() {
            panic!("Only owner can mint tokens");
        }

        state.total_supply += amount;
        let balance = state.balances.entry(env::caller()).or_insert(0);
        *balance += amount;

        env::set_storage(String::from("root"), state);
        env::commit(())
    }

    pub fn burn(amount: u128) {
        let mut state: TokenState = env::get_storage(String::from("root")).unwrap();

        let balance = state.balances.entry(env::caller()).or_insert(0);

        if *balance < amount {
            panic!("Not enough tokens to burn");
        }

        state.total_supply -= amount;
        *balance -= amount;

        env::set_storage(String::from("root"), state);
        env::commit(())
    }

    pub fn transfer(input: (AccountId, u128)) {
        let recipient = input.0;
        let amount = input.1;

        let mut state: TokenState = env::get_storage(String::from("root")).unwrap();

        let sender_balance = state.balances.entry(env::caller()).or_insert(0);
        if *sender_balance < amount {
            panic!("Not enough tokens to transfer");
        }
        *sender_balance -= amount;

        let recipient_balance = state.balances.entry(recipient).or_insert(0);
        *recipient_balance += amount;

        env::set_storage(String::from("root"), state);
        env::commit(())
    }

    pub fn transfer_from(input: (AccountId, u128)) {
        let sender = input.0;
        let amount = input.1;

        let mut state: TokenState = env::get_storage(String::from("root")).unwrap();

        let sender_balance = state.balances.entry(sender).or_insert(0);
        if *sender_balance < amount {
            panic!("Not enough tokens to transfer");
        }
        *sender_balance -= amount;

        let recipient_balance = state.balances.entry(env::caller()).or_insert(0);
        *recipient_balance += amount;

        env::set_storage(String::from("root"), state);
        env::commit(())
    }

    pub fn set_owner(new_owner: AccountId) {
        let mut state: TokenState = env::get_storage(String::from("root")).unwrap();

        if state.owner != env::caller() {
            panic!("Only owner can set new owner");
        }

        state.owner = new_owner;

        env::set_storage(String::from("root"), state);
    }

    pub fn get_owner() {
        let state: TokenState = env::get_storage(String::from("root")).unwrap();
        env::commit(state.owner);
    }
}
