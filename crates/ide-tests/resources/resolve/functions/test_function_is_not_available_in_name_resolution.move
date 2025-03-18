module 0x1::main {
    public fun call() { test_main(); }
                          //^ unresolved
    #[test]
    fun test_main() {}
}