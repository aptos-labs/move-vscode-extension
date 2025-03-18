module 0x1::m {
    struct S { f1: u8, f2: u8 }
             //X
    fun main(s: S) {
        let S { f1, .. } = s;
               //^
    }
}        