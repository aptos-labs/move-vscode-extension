module 0x1::M {
    spec schema MySchema {}
               //X
}
module 0x1::M2 {
    use 0x1::M;
    
    spec module {
        include M::MySchema;
                  //^
    }
}