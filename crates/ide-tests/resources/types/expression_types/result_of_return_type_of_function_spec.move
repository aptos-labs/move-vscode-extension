module 0x1::m {
    public fun get_fees_distribution(): u128 {
        1
    }
    spec get_fees_distribution {
        aborts_if false;
        ensures result == 1;
                 //^ num
    }
}