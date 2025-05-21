module 0x1::cast_expr_precedence {
    fun main() {
        1 / 2 as u8;
        (1 / 2 as u8);
    }
}
