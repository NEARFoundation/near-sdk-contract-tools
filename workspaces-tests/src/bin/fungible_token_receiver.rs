workspaces_tests::predicate!();

use near_sdk::{
    env, json_types::U128, log, near, AccountId, NearToken, PanicOnDefault, PromiseOrValue,
};
use near_sdk_contract_tools::ft::*;

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {}

#[near]
impl Nep141Receiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: near_sdk::AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        log!("Received {} from {}", amount.0, sender_id);

        if msg == "panic" {
            env::panic_str("panic requested");
        } else if let Some(account_id) = msg.strip_prefix("transfer:") {
            let account_id: AccountId = account_id.parse().unwrap();

            log!("Transferring {} to {}", amount.0, account_id);

            return ext_nep141::ext(env::predecessor_account_id())
                .with_attached_deposit(NearToken::from_yoctonear(1u128))
                .ft_transfer(account_id, amount, None)
                .then(Contract::ext(env::current_account_id()).return_value(amount)) // ask to return the token even though we don't own it anymore
                .into();
        }

        PromiseOrValue::Value(if msg == "return" { amount } else { U128(0) })
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    #[private]
    pub fn return_value(&self, value: U128) -> U128 {
        value
    }
}
