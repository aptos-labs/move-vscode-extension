module 0x1::m {
    fun call() {}
    spec call {
        forall i in 0..10: i < 20;
                         //^ num
    }
}        