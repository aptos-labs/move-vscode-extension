module 0x1::top_level_lemma_with_proof {
    spec lemma mul_comm(a: u64, b: u64) {
        ensures a * b == b * a;
    } proof {
        assume [trusted] true;
    }
}
