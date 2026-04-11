module 0x1::if_else_block_expr {
    fun main() {
        if (true) { 1 } else { 2 }
            || true
            || true
            || 1 + 1
    }
}
