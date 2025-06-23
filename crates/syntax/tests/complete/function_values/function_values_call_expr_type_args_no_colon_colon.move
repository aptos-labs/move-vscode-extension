module 0x1::function_values_call_expr_type_args_with_colon_colon {
    fun main() {
        let _g = pack(f).unpack<|| has drop + store + copy>();
        let g = pack(f).unpack<|| (|| has drop + store)>();
        let g = pack(f).unpack<||S<Xu64, X, u64> has drop + store>();
    }
}
