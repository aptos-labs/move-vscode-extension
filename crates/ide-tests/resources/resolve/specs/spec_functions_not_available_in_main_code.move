module 0x1::main {
    fun call() {
        spec_add();
        //^ unresolved
    }
}    
spec 0x1::main {
    spec fun spec_add(): u8 { 1 }
}