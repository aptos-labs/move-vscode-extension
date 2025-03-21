module 0x1::m {
    enum S { One { field: u8 }, Two { field: u8 } }
    fun main(s: S) {
        match (s) {
            One => s.field,
                    //^ u8
        }
    }
}        