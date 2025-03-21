module 0x1::m {
    public fun get_fees_distribution(): (u128, bool) {
        (1, false)
    }
    spec get_fees_distribution {
        aborts_if false;
        ensures result_2 == 1;
                 //^ bool
    }
}