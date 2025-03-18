module 0x1::mod {
    fun name() {}
       //X
    fun main() {
        let name = || 1;
        name();
         //^
    }
}        