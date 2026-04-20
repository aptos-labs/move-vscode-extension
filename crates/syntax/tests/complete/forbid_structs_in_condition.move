module 0x1::forbid_structs_in_condition {
    struct S has drop { value: bool }

    fun if_condition() {
        let s = S { value: true };
        if (s.value) { 1 } else { 2 };
    }

    fun while_condition() {
        let s = S { value: true };
        while (s.value) { };
    }
}