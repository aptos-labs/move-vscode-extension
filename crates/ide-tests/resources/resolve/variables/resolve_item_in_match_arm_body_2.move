module 0x1::m {
    enum S { One, Two }
    fun main(s: S) {
           //X
        let m = 1;
        match (m) {
            S::One => s
                    //^
        }
    }
}        