module 0x1::m {
    enum Outer { None { i: u8 } }
                //X

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            None { _ } => {}
            //^
        }
    }
}        