module 0x1::string {
          //X
    public fun utf8() {}
}
module 0x1::main {
    fun main() {
        use 0x1::string;
        string::utf8();
        //^
    }
}        