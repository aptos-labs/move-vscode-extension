module 0x1::main {
    fun call() {
    }
}    
spec 0x1::main {
    spec fun spec_add(): u8 { 1 }
             //X
    spec call {
        spec_add();
        //^
    }
}