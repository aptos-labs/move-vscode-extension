module 0x1::m {
    enum S { One { field: u8 }, Two }
    fun main(s: S) {
        match (s) {
            One { field: myfield } => myfield,
                                     //^ u8
        }
    }
}        