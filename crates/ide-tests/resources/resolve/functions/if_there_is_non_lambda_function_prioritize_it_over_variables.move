module 0x1::m {
    fun select_f(val: u8) {}
         //X
    fun main() {
        let select_f = 1;
        select_f(1);
        //^
    }
}