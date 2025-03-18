module 0x1::m {
    #[verify_only]
    fun call(): u8 { 1 }
       //X
    #[verify_only]
    fun main() {
        let _ = call();
               //^
    }
}        