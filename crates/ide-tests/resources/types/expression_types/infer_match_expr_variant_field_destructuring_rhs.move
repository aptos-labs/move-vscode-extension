module 0x1::m {
    enum S { One { field: u8 }, Two P }
    fun main(s: S) {
        match (s) {
            One { field } => field,
                               //^ u8
        }
    }
}        