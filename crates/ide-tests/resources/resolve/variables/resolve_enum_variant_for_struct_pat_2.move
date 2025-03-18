module 0x1::m {
    enum S { One { field: u8 }, Two }
           //X
    fun main(s: S::One) {
        let S::One { field } = s;
              //^
    }
}        