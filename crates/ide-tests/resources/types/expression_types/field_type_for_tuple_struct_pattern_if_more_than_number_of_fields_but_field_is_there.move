module 0x1::m {
    struct S(u8);
    fun main(s: S) {
        let S ( field1, field2 ) = s;
        field1;
        //^ u8
    }
}        