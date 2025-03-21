module 0x1::m {
    enum S1 { One, Two }
    fun main(s: S1) {
        let ret = s is One;
        ret;
      //^ bool 
    }
}        