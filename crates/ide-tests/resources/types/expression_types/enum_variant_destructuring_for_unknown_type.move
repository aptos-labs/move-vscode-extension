module 0x1::m {
    enum S {  }
    fun main() {
        let S::Inner { i } = s;
        i;
      //^ <unknown>  
    }
 }        