module 0x1::m {
    struct S has key {}
         //X
    fun main() {
        S[@0x1];
      //^   
    }
}        