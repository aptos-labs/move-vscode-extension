module 0x1::function_value_with_complex_type {
    fun main() {
        |v: |u8| (u8, |u8|)|;
        |
            v: | u8 | (Struct2, | bool | has copy + drop, Enum0, u16) has copy + drop
        | { 1 }
    }
}
