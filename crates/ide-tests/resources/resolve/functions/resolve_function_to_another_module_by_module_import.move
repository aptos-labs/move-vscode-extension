module 0x1::original {
    public fun call() {}
             //X
}
module 0x1::m {
    use 0x1::original;
    
    fun call() {}
    
    fun main() {
        original::call();
                //^
    }
}    