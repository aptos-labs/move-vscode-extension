module 0x1::M {
    fun call(addr: address) {}
    spec call {
        addr;
        //^ address
    }
}    