module 0x1::match_with_ranges {
    fun match_with_ranges(x: i32): u8 {
        match (x) {
            ..0  => 0,   // negative
            0..1    => 1,   // zero
            1..  => 2,   // positive
        }
    }
}
