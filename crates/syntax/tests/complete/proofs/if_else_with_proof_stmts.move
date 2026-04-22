module 0x1::if_else_with_proof_stmts {
    spec main() {} proof {
        if (true) { apply my_lemma(); } else { apply my_lemma(); }
        { apply my_lemma(); }
    }
}
