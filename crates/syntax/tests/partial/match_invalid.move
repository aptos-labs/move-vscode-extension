module 0x1::match_invalid {
    fun call(s: Enum0) {
        match (s) {
            (1)
        }
    }

    fun main(s: Enum0) {
        match (s) {
            (1);
        }
    }
}
