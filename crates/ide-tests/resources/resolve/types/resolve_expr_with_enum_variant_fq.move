module 0x1::m {
    enum S { One, Two }
           //X
    fun main() {
        let a: S = S::One;
                     //^
    }
}        