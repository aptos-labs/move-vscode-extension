module 0x1::main {
    fun call(a: u8, b: u8) {}
           //X
}
spec 0x1::main {
    spec call(a: u8, b: u8) {}
            //^
}