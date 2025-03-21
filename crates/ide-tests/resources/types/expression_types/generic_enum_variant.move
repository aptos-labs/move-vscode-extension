module 0x1::m {
    enum S<phantom T> { One }
    fun main() {
        let a = S<u8>::One;
        a;
      //^ 0x1::m::S<u8>
    }
}        