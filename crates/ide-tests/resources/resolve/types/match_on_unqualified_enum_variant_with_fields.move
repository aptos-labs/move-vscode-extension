module 0x1::m {
    enum S { One { field: u8 }, Two }
            //X
    fun main(s: S): bool {
        match (s) {
            One { field: _ } => true
           //^ 
        }
    }
}        