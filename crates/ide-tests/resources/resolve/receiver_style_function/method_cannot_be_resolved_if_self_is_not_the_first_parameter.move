module 0x1::main {
    struct S { x: u64 }
    fun receiver(y: u64, self: &S): u64 {
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver(&s)
          //^ unresolved
    }
}                