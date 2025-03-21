module 0x1::M {
    struct S {}
    fun ref(): &S { &S {} }
    spec module {
        let a = ref();
        a;
      //^ 0x1::M::S
    }
}    