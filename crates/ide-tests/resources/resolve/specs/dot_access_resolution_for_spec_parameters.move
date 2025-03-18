module 0x1::M {
    struct S { val: u8 }
             //X
    spec schema SS {
        s: S;
        s.val;
        //^
    }
}    