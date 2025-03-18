module 0x1::s {
    struct MyStruct {}
            //X
}
module 0x1::m {
    use 0x1::s::MyStruct;
                //^
}