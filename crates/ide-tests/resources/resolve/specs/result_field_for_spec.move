module 0x1::main {
    struct S { val: u8 }
               //X
    fun call(): S { S { val: 1 } }
    spec call {
        ensures result.val == 1;
                      //^
    }
}        