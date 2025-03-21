module 0x1::m {
    enum Option<T> { Some { element: T } }
    fun main() {
        let a = Option::Some { element: 1u8 };
        a;
      //^ 0x1::m::Option<u8>  
    }
}        