module 0x1::lambda_with_incorrect_pattern {
    fun main() {
        |&a, _, 22, d| 1;
    }
}
