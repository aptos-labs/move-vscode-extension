module 0x1::let_expr_invalid_pattern {
    fun main() {
        let &self.vec[0]
    }
    spec main {}
}
