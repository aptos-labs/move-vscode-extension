module 0x1::m {
    enum S { One { field: u8 }, Two }
           //X
    fun main() {
        let a: S = S::One { field: 1 };
                     //^
    }
}        