module 0x1::M {
    spec schema SS<Type> {}
    spec module {
        apply SS<Type>
                 //^
            to *<Type>;
                 //X
    }
}    