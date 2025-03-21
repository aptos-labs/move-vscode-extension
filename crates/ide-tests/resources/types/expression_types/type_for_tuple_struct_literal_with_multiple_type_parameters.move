module 0x1::m {
    struct S<T, U>(T, U);
    fun main() {
        let s = S(true, 1u8);
        s; 
      //^ 0x1::m::S<bool, u8>   
    }
}        