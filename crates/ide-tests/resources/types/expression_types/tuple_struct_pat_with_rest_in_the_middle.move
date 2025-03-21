module 0x1::m {
    struct S(u8, u8, u8, bool);
    fun main(s: S) {
        let S(f1, .., fb) = s;
        fb;
       //^ bool
    }
}        