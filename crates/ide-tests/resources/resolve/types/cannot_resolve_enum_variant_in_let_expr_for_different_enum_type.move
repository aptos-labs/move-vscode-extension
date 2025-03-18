module 0x1::m {
    enum S1 { One, Two }
    enum S2 { }
    fun main(_: S1) {
        let s: S2 = One;
                   //^ unresolved
    }
}        