use near_sdk::{
    borsh, collections::Vector, env, json_types::U128, log, near, test_utils::VMContextBuilder,
    testing_env, AccountId, NearToken, PanicOnDefault, PromiseOrValue,
};
use near_sdk_contract_tools::{hook::Hook, standard::nep141::*, Nep141};

#[derive(Nep141, PanicOnDefault)]
#[nep141(transfer_hook = "TransferHook")]
#[near(contract_state)]
struct FungibleToken {
    pub transfers: Vector<Vec<u8>>,
    pub hooks: Vector<String>,
}

#[derive(Default)]
struct TransferHook;

impl Hook<FungibleToken, Nep141Transfer<'_>> for TransferHook {
    fn hook<R>(
        contract: &mut FungibleToken,
        args: &Nep141Transfer,
        f: impl FnOnce(&mut FungibleToken) -> R,
    ) -> R {
        let storage_usage_start = env::storage_usage();
        contract.hooks.push(&"before_transfer".to_string());
        let r = f(contract);
        contract.hooks.push(&"after_transfer".to_string());
        contract.transfers.push(&borsh::to_vec(&args).unwrap());
        let storage_usage_end = env::storage_usage();
        println!("Storage delta: {}", storage_usage_end - storage_usage_start);
        r
    }
}

mod receiver {
    use super::*;

    #[derive(PanicOnDefault)]
    #[near(contract_state)]
    struct FungibleTokenReceiver {
        pub log: Vector<(String, u128)>,
    }

    impl near_sdk_contract_tools::standard::nep141::Nep141Receiver for FungibleTokenReceiver {
        fn ft_on_transfer(
            &mut self,
            sender_id: AccountId,
            amount: U128,
            msg: String,
        ) -> PromiseOrValue<U128> {
            let used_amount: u128 = amount.0 / 2;

            let out = format!("ft_on_transfer[from={sender_id}, used={used_amount}]");
            log!(&out);
            println!("{out}");

            self.log.push(&(msg, amount.0));

            PromiseOrValue::Value(U128(used_amount))
        }
    }
}

#[test]
fn nep141_transfer() {
    let mut ft = FungibleToken {
        transfers: Vector::new(b"t"),
        hooks: Vector::new(b"h"),
    };

    let alice: AccountId = "alice".parse().unwrap();
    let bob: AccountId = "bob".parse().unwrap();

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 0);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 0);
    assert_eq!(ft.ft_total_supply().0, 0);

    ft.deposit_unchecked(&alice, 100).unwrap();
    ft.deposit_unchecked(&bob, 20).unwrap();

    assert_eq!(ft.transfers.pop(), None);
    assert_eq!(ft.ft_balance_of(alice.clone()).0, 100);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 20);
    assert_eq!(ft.ft_total_supply().0, 120);

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .attached_deposit(NearToken::from_yoctonear(1u128))
        .build();

    testing_env!(context);

    ft.ft_transfer(bob.clone(), 50.into(), None);

    assert_eq!(
        ft.transfers.pop(),
        Some(borsh::to_vec(&Nep141Transfer::new(50, alice.clone(), bob.clone())).unwrap())
    );

    let expected_hook_execution_order = vec!["before_transfer", "after_transfer"];
    let actual_hook_execution_order = ft.hooks.to_vec();
    assert_eq!(expected_hook_execution_order, actual_hook_execution_order);

    assert_eq!(ft.ft_balance_of(alice).0, 50);
    assert_eq!(ft.ft_balance_of(bob).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}
