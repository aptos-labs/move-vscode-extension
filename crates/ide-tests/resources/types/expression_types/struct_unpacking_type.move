module 0x1::m {
    struct S<CoinType> { amount: CoinType }
    fun call<CallCoinType>(s: S<CallCoinType>) {
        let S { amount: my_amount } = s;
        my_amount;
        //^ CallCoinType
    }
}               