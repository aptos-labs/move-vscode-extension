module 0x1::main {
    struct S { val: u8 }
    fun call(): S { S { val: 1 } }
    spec call {
        ensures result.val == 1;
                      //^ num
    }
}        