module 0x1::m {
    enum Option { None }
    fun main() {
        let a = Option::None;
        a;
      //^ 0x1::m::Option  
    }
}        