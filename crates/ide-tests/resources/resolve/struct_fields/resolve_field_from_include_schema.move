module 0x1::m {
    struct S { val: u8 }
              //X
    spec schema MySchema {
        schema_val: u8;
    }
    spec module {
        let s = S { val: 10 };
        include MySchema {
            schema_val: s.val
                         //^
        };
    }
}        