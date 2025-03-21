module 0x1::m {
    fun call<T>(t: T): T { t }
    fun main() {
        let a = call<u8>(1);
        a;
      //^ u8  
    }
}        