module 0x1::m {
    #[test_only]
    fun call(): u8 { 1 }
    fun main() {
        let _ = call();
               //^ unresolved
    }
}