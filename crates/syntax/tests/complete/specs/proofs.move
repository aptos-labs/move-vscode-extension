module 0x1::proofs {
    spec main {} proof {
        apply my_lemma(1, 2, 3);
        assert true;
        assume [trusted] 1 + 1 == 2;
    }
}
