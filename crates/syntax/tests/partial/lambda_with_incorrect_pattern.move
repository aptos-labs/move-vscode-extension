module 0x1::lambda_with_incorrect_pattern {
    fun main() {
        |&a, _, d, 22| 1;
    }
}
