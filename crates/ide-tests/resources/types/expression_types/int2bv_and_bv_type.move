module 0x1::m {
    fun call() {}
    spec call {
        let a = int2bv(100);
        a;
      //^ bv  
    }
}        