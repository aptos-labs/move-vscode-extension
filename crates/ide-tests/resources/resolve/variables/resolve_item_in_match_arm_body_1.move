module 0x1::m {
    enum S { One, Two }
    fun main() {
        let m = 1;
          //X
        match (m) {
            S::One => m
                    //^
        }
    }
}        