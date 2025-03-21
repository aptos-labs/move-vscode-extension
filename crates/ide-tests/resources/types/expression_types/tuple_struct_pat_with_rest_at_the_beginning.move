module 0x1::m {
    struct S(u8, u8, bool);
    fun main(s: S) {
        let S(.., f1) = s;
        f1;
       //^ bool
    }
}        