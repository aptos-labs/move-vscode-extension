module 0x1::main {
    struct S has key { val: u8 }
                      //X
}
spec 0x1::main {
    spec fun spec_now() {
        global<S>(@0x1).val;
                       //^ 
    }
} 