module 0x1::m {
    #[verify_only]
    fun call(): u8 { 1 }
    fun main() {
        let _ = call();
               //^ unresolved
    }
}