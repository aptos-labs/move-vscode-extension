module 0x1::m {
    enum S { One, Two }
            //X
    fun main(s: S): bool {
        match (s) {
            S::One => true
              //^ 
        }
    }
}        