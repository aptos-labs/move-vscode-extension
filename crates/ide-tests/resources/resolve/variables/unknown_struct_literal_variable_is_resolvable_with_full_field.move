module 0x1::m {
    fun main() {
        let myfield = 1;
             //X
        Unknown { field: myfield };
                        //^
    }
}        