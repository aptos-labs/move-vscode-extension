module 0x1::m {
    enum S1 { One { field: u8 }, Two }
             //X
    enum S2 { One { field: u8 }, Two }
    fun main(s: S1) {
        match (s) {
            One { field } => field,
            //^ 
        }
    }
}        