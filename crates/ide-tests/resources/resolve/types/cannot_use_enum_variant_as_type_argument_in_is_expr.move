module 0x1::m {
    enum S1<T> { One, Two }
    enum S2 { Inner }
    fun main(s: S1) {
        if (s is S1<S2::Inner>::One) true;
                       //^ unresolved
    }
}        