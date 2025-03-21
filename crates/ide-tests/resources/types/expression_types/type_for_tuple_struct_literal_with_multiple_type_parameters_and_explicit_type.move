module 0x1::m {
    struct S<T>(T, T);
    fun main() {
        let s = S<u8>(1, 1);
        s; 
      //^ 0x1::m::S<u8>   
    }
}        