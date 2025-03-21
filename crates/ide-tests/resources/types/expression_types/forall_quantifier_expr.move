module 0x1::m {
    fun call() {}
    spec call {
        let a = forall i in 0..10: i < 20;
        a;
      //^ bool  
    }
}        