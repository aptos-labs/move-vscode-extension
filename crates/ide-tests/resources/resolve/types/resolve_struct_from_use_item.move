module 0x1::M {
    struct MyStruct {}
           //X
}    
module 0x1::Main {
    use 0x1::M::{Self, MyStruct};
                      //^
}