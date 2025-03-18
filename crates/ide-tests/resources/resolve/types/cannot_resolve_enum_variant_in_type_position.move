module 0x1::m {
    enum S { One, Two }
    fun main(one: S::One) {
                    //^ unresolved
    }
}        