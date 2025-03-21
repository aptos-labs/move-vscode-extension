module 0x1::m {
    inline fun main<Element>(e: Element, f: |Element| u8) {
        f(e);
      //^ u8  
    }
}        