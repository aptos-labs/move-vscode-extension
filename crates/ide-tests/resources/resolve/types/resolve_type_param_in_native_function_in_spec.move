module 0x1::M {
    spec module {
        native fun serialize<MoveValue>(
                                //X
            v: &MoveValue
                //^
        ): vector<u8>;
    }
}    