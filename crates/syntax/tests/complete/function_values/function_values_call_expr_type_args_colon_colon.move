module 0x1::function_values_call_expr_type_args_colon_colon {
    fun main() {
        let _g = pack(f).unpack::<|| has drop + store + copy>();
    }
}
