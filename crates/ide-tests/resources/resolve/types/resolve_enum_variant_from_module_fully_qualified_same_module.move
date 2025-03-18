module 0x1::m {
    enum S { One, Two }
           //X
    fun main() {
        let s = 0x1::m::S::One;
                           //^
    }
}        