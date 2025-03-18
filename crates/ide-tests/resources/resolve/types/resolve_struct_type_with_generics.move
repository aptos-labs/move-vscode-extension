module 0x1::m {
    struct Native<T> {}
         //X
    fun main(n: Native<u8>): u8 {}
              //^
}