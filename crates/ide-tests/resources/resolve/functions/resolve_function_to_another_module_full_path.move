module 0x1::original {
    public fun call() {}
               //X
}
module 0x1::m {
    fun call() {}
    
    fun main() {
        0x1::original::call();
                     //^
    }
}