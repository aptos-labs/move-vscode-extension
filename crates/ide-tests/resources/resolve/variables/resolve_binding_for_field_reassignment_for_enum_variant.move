module 0x1::m {
    enum S { One { field: u8 }, Two }
    fun main() {
        let m = 1;
        match (m) {
            S::One { field: myfield }
                            //X
                => myfield
                    //^
        }
    }
}        