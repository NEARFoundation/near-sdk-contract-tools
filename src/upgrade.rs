//! Upgrade Mod
//!
//! Makes it easier to upgrade your contract by providing a simple interface for upgrading the code and the state of your contract.

use near_sdk::{env, sys, Gas};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// Upgrade Trait
pub trait Upgrade {
    /// upgrade_all - Upgrades the code and the state of the contract
    fn upgrade_all();
}

/// Contracts may implement this trait to inject code into Upgrade functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait UpgradeHook {
    /// Executed before a upgrade call is conducted
    fn on_upgrade();
}

/// naked upgrade function which calls migrate method on the contract
pub fn upgrade<T>()
where
    T: BorshDeserialize + BorshSerialize,
{
    env::setup_panic_hook();

    const MIGRATE_METHOD_NAME: &[u8; 7] = b"migrate";
    const UPDATE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

    unsafe {
        // Load code into register 0 result from the input argument if factory call or from promise if callback.
        sys::input(0);
        // Create a promise batch to update current contract with code from register 0.
        let promise_id = sys::promise_batch_create(
            env::current_account_id().as_bytes().len() as u64,
            env::current_account_id().as_bytes().as_ptr() as u64,
        );
        // Deploy the contract code from register 0.
        sys::promise_batch_action_deploy_contract(promise_id, u64::MAX, 0);
        // Call promise to migrate the state.
        // Batched together to fail upgrade if migration fails.
        sys::promise_batch_action_function_call(
            promise_id,
            MIGRATE_METHOD_NAME.len() as u64,
            MIGRATE_METHOD_NAME.as_ptr() as u64,
            0,
            0,
            0,
            (env::prepaid_gas() - env::used_gas() - UPDATE_GAS_LEFTOVER).0,
        );
        sys::promise_return(promise_id);
    }
}