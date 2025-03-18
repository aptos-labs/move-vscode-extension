module 0x1::m {
    enum S { One, Two }
    fun consume() {}
       //X
    fun main(s: S): bool {
        match (s) {
            S::One if consume() => true
                      //^
        }
    }
}        