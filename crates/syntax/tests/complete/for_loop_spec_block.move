module 0x1::for_loop_spec_block {
    fun main() {
        for (i in 0..10) {
        } spec {
            invariant i <= 11;
        };
    }
}
