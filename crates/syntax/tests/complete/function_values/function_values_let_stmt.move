module 0x1::function_values_let_stmt {
    fun main() {
        let f: |u64|bool has copy = |x| x > 0;
    }
}
