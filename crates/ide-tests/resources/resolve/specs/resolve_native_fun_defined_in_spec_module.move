module 0x1::m {
    spec module {
        native fun serialize<MoveValue>(v: &MoveValue): vector<u8>;
                    //X
    }
}
module 0x1::main {
    use 0x1::m;
    spec module {
        m::serialize(&true);
           //^
    }
}