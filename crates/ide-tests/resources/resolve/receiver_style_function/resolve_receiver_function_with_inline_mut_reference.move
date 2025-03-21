module 0x1::main {
    struct S { x: u64 }
    inline fun receiver_mut_ref(self: &mut S, y: u64): u64 {
              //X
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver_mut_ref(1)
          //^
    }
}        