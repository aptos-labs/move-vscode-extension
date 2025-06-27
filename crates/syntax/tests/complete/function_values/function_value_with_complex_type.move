module 0x1::function_value_with_complex_type {
    fun main() {
        let s: | bool | (u8, | bool | has copy + drop, u16) has copy + drop;
    }
}
