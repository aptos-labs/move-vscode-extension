module 0x1::proofs {
    spec main {} proof {
        let a = 1;
    }
    spec module {} proof {}

    spec main {}
    proof {
        if (n > 0) {
            apply f_pos(n - 1);
        }
    }
}
