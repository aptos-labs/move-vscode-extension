module 0x1::m {
    enum Inner { Inner1, Inner2 }
                //X
    enum Outer { One { inner: Inner } }
    
    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner: Inner1 } => Inner1
                         //^
        }
    }
}        