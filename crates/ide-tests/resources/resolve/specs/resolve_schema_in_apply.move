module 0x1::M {
    spec schema MySchema {}
                //X
    spec module {
        apply MySchema to *;
              //^
    }
}    