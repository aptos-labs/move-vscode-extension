module 0x1::main {
    fun call(a: vector<u8>) {}
    fun main() {
        let vv = vector[];
        call(vv);
        vv;
       //^ vector<u8>   
    }
}        