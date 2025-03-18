module 0x1::m {
    entry fun call() {}
              //X
}
module 0x1::main {
    use 0x1::m;
    entry fun main() {
        m::call();
           //^ unresolved
    }
}