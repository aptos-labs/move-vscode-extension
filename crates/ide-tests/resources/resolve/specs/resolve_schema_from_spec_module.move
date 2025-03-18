module 0x1::call {}
spec 0x1::call {
    spec schema MySchema {}
                //X
}
module 0x1::main {
    use 0x1::call::MySchema;
    
    spec module {
        include MySchema;
                //^
    }
}