module 0x1::M {
    public fun call() {}
             //X
    fun main() {
        0x1::M::call();
                //^  
    }
}