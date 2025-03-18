module 0x1::m {
    enum S { One, Two }
       //X
    fun main(one: S::One) {
                //^
    }
}        