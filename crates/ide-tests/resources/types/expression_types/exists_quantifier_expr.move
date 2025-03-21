module 0x1::m {
    fun call() {}
    spec call {
        let a = exists i in 0..10: i == 1;
        a;
      //^ bool  
    }
}        