module 0x1::m {
    enum S { One { field: u8 }, Two }
    fun main(s: S::One) {
        let f = 1;
          //X
        let s = S::One { field: f };
                              //^
    }
}        