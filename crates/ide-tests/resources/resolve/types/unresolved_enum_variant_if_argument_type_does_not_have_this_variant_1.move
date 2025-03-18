module 0x1::m {
    enum S1 { One, Two }
    enum S2 {}
    fun main(s: S2) {
        match (s) {
            One => true,
            //^ unresolved
        }
    }
}        