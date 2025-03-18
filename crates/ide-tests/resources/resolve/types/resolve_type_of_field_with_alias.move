module 0x1::m {
    struct S { field: u8 }
}
module 0x1::main {
    use 0x1::m::S as MyS;
                    //X
    struct R { field: MyS }
                     //^
}        