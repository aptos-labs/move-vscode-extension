module 0x1::m {
    enum Outer { None }
                //X

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            None => {}
            //^
        }
    }
}        