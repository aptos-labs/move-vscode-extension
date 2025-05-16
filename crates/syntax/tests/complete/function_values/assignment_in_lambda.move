module 0x1::assignment_in_lambda {
    fun main() {
        |elem| accu = f(accu, elem);
        self.for_each(|elem| accu = f(accu, elem));
    }
}
