module 0x1::M {
    spec module {
        include MySchema;
                 //^
    }
    spec schema MySchema {}
                //X
}    