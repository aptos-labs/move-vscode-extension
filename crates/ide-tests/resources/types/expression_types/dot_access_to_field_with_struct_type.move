module 0x1::M {
    struct Addr {}
    struct S { addr: Addr }
    fun main() {
        let s = S { addr: Addr {} };
        ((&s).addr);
      //^ 0x1::M::Addr 
    }
}    