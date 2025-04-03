module 0x1::incomplete_lambda_exr {
    fun main() {
        let a = self.all(|_| { 1 + 1 });
    }
}
