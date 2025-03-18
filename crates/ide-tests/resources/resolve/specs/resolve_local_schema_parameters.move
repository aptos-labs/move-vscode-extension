module 0x1::M {
    spec module {
        let a = @0x1;
        include MySchema { addr: a };
                          //^
    }
    
    spec schema MySchema {
        local addr: address;
             //X
    }
}    