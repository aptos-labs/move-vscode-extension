module 0x1::m {
    enum S { One { field: u8 }, Two }
           //X
    enum T { One { field: u8 }, Two }
    fun main() {
        let a: S = One { field: 1 };
                  //^
    }
}        