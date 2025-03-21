module 0x1::M {
    struct S {}
    fun m(s: &S, s_mut: &mut S) {
        let cond = true;
        (if (cond) s_mut else s);
      //^ &0x1::M::S  
    }
}    