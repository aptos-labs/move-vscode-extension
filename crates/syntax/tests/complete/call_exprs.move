module 0x1::call_exprs {
    fun main() {
        call();
        a.method();
        a.method::<u8>();
    }
}
