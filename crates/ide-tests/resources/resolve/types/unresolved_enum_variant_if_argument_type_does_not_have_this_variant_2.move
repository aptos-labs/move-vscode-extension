module 0x1::m {
    enum S1 { One { field: u8 }, Two }
    enum S2 {}
    fun main(s: S2) {
        match (s) {
            One { field } => true,
            //^ unresolved
        }
    }
}        