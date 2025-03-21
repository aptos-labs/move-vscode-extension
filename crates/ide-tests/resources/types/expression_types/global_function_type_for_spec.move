module 0x1::M {
    struct S has key {}
    spec module {
        let a = global<S>(@0x1);
        a;
      //^ 0x1::M::S 
    }
}    