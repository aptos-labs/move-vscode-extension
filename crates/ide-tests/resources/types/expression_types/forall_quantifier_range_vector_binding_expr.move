module 0x1::m {
    fun call() {}
    spec call {
        forall i in vector[true, false]: i;
                                       //^ bool
    }
}        