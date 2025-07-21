module 0x1::missing_expr_in_struct_lit {
    fun main() {
        Any { val: };
        Any { val: 1+ };
    }
}
