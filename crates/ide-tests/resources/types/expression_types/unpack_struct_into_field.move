    module 0x1::M {
    struct S { val: u8 }
    fun s(): S { S { val: 10 } }
    fun main() {
        let s = s();
        s;
      //^ 0x1::M::S   
    }
}            