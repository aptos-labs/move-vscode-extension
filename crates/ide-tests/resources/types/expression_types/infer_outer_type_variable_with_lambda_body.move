module 0x1::vector {
    public native fun for_each_ref<Element>(self: &vector<Element>, f: |&Element|);
    public native fun contains<Element>(self: &vector<Element>, e: &Element): bool;
}
module 0x1::account {
    use 0x1::vector;
    struct Account { account_address: address }
    fun create_accounts(accounts: vector<Account>) {
        let unique_accounts = vector[];
        accounts.for_each_ref(|account| {
            assert!(
                !vector::contains(&unique_accounts, &account.account_address),
                1,
            );
        });
        unique_accounts;
        //^ vector<address>
    }
}        