module 0x1::M {
    struct S {}
    fun call(a: S) {}
    spec call {
        old(a);
      //^ 0x1::M::S 
    }
}    