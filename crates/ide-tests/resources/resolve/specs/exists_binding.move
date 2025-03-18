module 0x1::M {
    spec module {
        invariant exists addr: address
                       //X
            : addr != @0x1;
            //^
    }
}    