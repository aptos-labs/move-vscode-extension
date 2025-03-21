module 0x1::main {
    struct S { val: S }
    fun main() {
        let s = S { val: };
        s;
      //^ 0x1::main::S  
    }
}        