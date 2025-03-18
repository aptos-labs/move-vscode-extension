module 0x1::M {
    spec schema MySchema {
        local ensures: address;
             //X
        update ensures = 1;
              //^  
    }
}    