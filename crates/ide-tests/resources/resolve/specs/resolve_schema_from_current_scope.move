module 0x1::Module {}    
spec 0x1::Module {
    spec schema MySchema {
               //X
    }
    spec schema MySchema2 {
        include MySchema;
                 //^
    }
}