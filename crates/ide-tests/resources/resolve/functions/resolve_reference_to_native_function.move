module 0x1::m {
    native fun call(): u8;
             //X
    
    fun main() {
        call();
      //^
    }
}