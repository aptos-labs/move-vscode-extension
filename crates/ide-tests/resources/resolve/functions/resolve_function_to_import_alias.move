module 0x1::original {
    public fun call() {}
}    
module 0x1::m {
    use 0x1::original::call as mycall;
                             //X
    fun main() {
        mycall();
      //^  
    }
}    