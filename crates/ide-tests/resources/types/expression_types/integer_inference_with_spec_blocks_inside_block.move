module 0x1::main {
    spec fun get_num(): num { 1 }
    fun main() {
        let myint = 1;
        myint + 1u8;
        spec {
            myint
            //^ num
        };
    }
}    