module 0x1::m {
    public fun call() {}
             //X
    fun main() {
        0x1::m::call();
              //^
    }
}