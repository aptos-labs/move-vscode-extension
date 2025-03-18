module 0x1::m {
    struct S(u8, u8);
         //X
    fun main(s: S) {
        let S ( field1, field2 ) = s;
          //^
    }
}        