module 0x1::M {
    spec schema MySchema {}
                 //X
    spec module {
        include if (true) MySchema else MySchema;
                                        //^
    }
}    