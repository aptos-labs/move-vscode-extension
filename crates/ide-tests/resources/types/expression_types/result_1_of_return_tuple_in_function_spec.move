module 0x1::m {
    public fun get_fees_distribution(): (u128, u128) {
        (1, 1)
    }
    spec get_fees_distribution {
        aborts_if false;
        ensures result_1 == 1;
                 //^ num
    }
}