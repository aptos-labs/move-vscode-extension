module 0x1::m {
    struct S(u8);
    fun main(s: S) {
        s.0;
        //^ u8
    }
}        