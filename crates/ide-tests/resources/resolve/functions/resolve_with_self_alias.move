module 0x1::string {
    public fun utf8() {}
              //X
}
module 0x1::main {
    use 0x1::string::Self as mystring;
    fun main() {
        mystring::utf8();
                //^
    }
}        