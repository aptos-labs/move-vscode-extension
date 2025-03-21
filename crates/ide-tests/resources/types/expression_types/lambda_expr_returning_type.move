module 0x1::m {
    inline fun main<Element>(f: |Element| Element) {
        f();
      //^ Element  
    }
}        