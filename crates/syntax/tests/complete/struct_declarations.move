module 0x1::M {
    struct Coin<phantom CoinType, phantom CoinType2> {}

    struct ValidatorConfig<T> has store, drop, copy {
        val1: u8,
        val2: vector<T>,
    }

    struct ResourceValidatorConfig<T: copy, U: copy + drop> {
        val1: u8,
        val2: vector<T>,
        operator_account: Option<address>,
        operator_account2: Option<&signer>,
    }
}
