module 0x1::m {
    inline fun main<Element>(f: |Element|) {
        let g = f;
        g;
      //^ |Element| -> ()  
    }
}        