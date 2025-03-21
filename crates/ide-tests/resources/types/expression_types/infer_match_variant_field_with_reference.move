module 0x1::m {
    struct Inner { field: u8 }
    enum Outer { None { inner: Inner } }

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            None { inner: myinner } => myinner
                                        //^ &0x1::m::Inner
        }
    }
}        