module 0x1::M {
    struct S { addr: address }
    fun main() {
        let s = S { addr: @0x1 };
        ((&s).addr);
      //^ address 
    }
}    