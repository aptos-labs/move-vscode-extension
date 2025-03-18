module 0x1::m {
    struct S(u8);
         //X
    fun main() {
        S(1);
      //^  
    }
}        