module 0x1::m {
    struct S { x: u64 }
    public(package) fun receiver(self: &S): u64 { self.x }
                          //X
}
module 0x1::main {
    use 0x1::m::S;
    fun main(s: S) {
        s.receiver();
          //^
    }
}        