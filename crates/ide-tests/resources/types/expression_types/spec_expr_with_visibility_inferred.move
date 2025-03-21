module 0x1::m {
    spec module {
        let i = false;
        invariant [suspendable] i;
                              //^ bool
    }
}        