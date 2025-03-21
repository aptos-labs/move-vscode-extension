module 0x1::M {
    struct S {}
    fun ref_mut(): &mut S {}
    spec module {
        let a = ref_mut();
        a;
      //^ 0x1::M::S
    }
}    