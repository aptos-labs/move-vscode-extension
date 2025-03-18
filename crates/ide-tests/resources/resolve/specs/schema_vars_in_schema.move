module 0x1::M {
    spec schema MySchema {
        var1: address;
        //X
        var1;
       //^  
    }
}    