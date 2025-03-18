module 0x1::m {
    #[verify_only]
    fun call(): u8 { 1 }
       //X
    fun main() {}
    spec main {
        let _ = call();
               //^
    }
}        