#[test_only]
module 0x1::attributes {
    #[test_only]
    #[test, expected_failure = std::type_name::ENonModuleType]
    #[attr1, attr2]
    #[test(a = @0x1, b = @0x2, c = @Std)]
    #[test()]
    #[show(book_orders_sdk, book_price_levels_sdk)]
    #[expected_failure(abort_code = liquidswap::liquidity_pool::ERR_ADMIN, location=0x1::liquidity_pool)]
    #[allow(lint(self_transfer))]
    #[expected_failure(
        abort_code = liquidity_pool::ERR_ADMIN,
        location = aptos_framework::ed25519,
        location = aptos_framework::ed25519::myfunction,
    )]
    #[lint::allow_unsafe_randomness]
    fun main() {}
}
