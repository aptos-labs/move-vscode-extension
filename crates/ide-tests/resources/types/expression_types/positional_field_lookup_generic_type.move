module 0x1::m {
    struct S<T>(T)
    fun main() {
        let s = S(true);
        s.0;
        //^ bool
    }
}        