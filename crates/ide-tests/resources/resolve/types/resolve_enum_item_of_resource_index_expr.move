module 0x1::m {
    enum Ss has key { Empty }
       //X
    fun main() {
        &mut Ss[@0x1];
           //^
    }
}        