module 0x1::m {
    fun call() {}
    spec call {
        forall i: num : i < 20;
                      //^ num
    }
}        