module 0x1::function_values_lambda_abilities {
    fun main(
        a: |bool| SettleTradeResult has drop + copy,
        a: |bool, address| Option<u64> has drop + copy,
    ) {
    }
}
