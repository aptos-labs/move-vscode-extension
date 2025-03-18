module 0x1::m {
    struct Inner { field: u8 }
                     //X
    enum Outer { One { inner: Inner } }
    
    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner: Inner { field: myfield } } => myfield
                                //^
        }
    }
}        