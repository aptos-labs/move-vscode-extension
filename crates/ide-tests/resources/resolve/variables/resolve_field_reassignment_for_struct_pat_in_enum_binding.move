module 0x1::m {
    enum S { One { field: u8 }, Two }
    fun main(s: S::One) {
        let S::One { field: f } = s;
                          //X
        f;
      //^  
    }
}        