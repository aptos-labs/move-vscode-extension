module 0x1::M {
          //X
    struct MyStruct {}
}    
module 0x1::Main {
    use 0x1::M::{Self, MyStruct};
                //^
}