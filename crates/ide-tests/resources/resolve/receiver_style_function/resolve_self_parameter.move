module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
                 //X
        self.x + y
        //^
    }
}        