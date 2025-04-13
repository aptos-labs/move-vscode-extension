module 0x1::lambda_expr_in_method {
    fun main() {
        self.add(|a, b| { 1 + 1 });
    }
}
