module 0x1::M {
    struct S { vec: vector<u8> } 
    spec module {
        let s = S { vec: b"" };
        s.vec
        //^ vector<num>
    }
}    