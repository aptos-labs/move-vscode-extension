module 0x1::main {
    fun call<T>(a: u8, b: u8) {}
           //X
}
spec 0x1::main {
    spec call<T>(a: u8, b: u8) {}
            //^
}