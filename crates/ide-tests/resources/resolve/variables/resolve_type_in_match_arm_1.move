module 0x1::m {
    enum S { One, Two }
       //X
    fun main() {
        let m = 1;
        match (m) {
            S::One => true
          //^  
            S::Two => false
        }
    }
}        