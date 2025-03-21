module 0x1::m {
    enum S { One, Two }
    fun main(s: S) {
        match (s) {
            S::One => s
                    //^ 0x1::m::S
        }
    }
}        