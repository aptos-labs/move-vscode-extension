module 0x1::m {
    spec module {
        let v = vector[true, false];
        let slice = v[0..1];
        slice;
        //^ vector<bool>
    }
}        