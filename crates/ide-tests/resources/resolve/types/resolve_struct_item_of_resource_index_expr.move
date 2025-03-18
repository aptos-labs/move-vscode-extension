module 0x1::m {
    struct Ss has key { val: u8 }
          //X
    fun main() {
        &mut Ss[@0x1];
           //^
    }
}        