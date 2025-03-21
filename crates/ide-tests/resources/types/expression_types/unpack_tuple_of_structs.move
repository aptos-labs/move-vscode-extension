    module 0x1::M {
    struct S { val: u8 }
    fun s(): (S, S) { (S { val: 10 }, S { val: 10 }) }
    fun main() {
        let (s, t) = s();
        s;
      //^ 0x1::M::S   
    }
}            