module 0x1::m {
    fun call() {}
    spec call {
        let a = bv2int(int2bv(100));
        a;
      //^ num  
    }
}        