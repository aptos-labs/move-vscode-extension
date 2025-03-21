module 0x1::m {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
        self.x + y
    }
}
module 0x1::main {
    use 0x1::m::S;
    
    fun test_call_styles(s: S): u64 {
        s.receiver(1)
          //^ unresolved
    }
}