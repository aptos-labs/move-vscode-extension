module 0x1::m {
    enum S<T> { One(T, T) }
    fun main() {
        let a = S<u8>::One(1, 1);
        a;
      //^ 0x1::m::S<u8>  
    }
}        