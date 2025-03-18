module 0x1::m {
    spec module {
            // `deserialize` is an injective function.
        axiom<T> forall b1: vector<u8>, b2: vector<u8>:
            //X
            (deserialize<T>(b1) == deserialize<T>(b2) ==> b1 == b2);
                       //^
    }
}        