module 0x1::match_with_ints {
    fun match_with_ints() {
        match (200) {
            0 => b"0",
            -1 => b"-1",
            200 => b"OK",
            404 => b"Not Found",
            _   => b"Unknown",
        };
    }
}
