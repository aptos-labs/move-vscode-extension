module 0x1::m {
    fun call<Element>(v: Element): Element {}
    fun main() {
        let u = 1;
        let (a, b): (u8, u8);
        (a, b) = (call(u), call(u));
        u;
      //^ u8  
    }        
} 