module 0x1::m1 {
    friend 0x1::main;
}
module 0x1::m2 {
    friend fun call() {}
}
module 0x1::main {
    use 0x1::m2;
    fun main() {
        m2::call();
           //^ unresolved
    }
}