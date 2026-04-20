module 0x1::forbid_structs_in_range {
    fun range_then_block() {
        let _v = 0..10;
        let _v = 0..{ 10 };
    }
}