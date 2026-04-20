module 0x1::lemmas {
    spec module {
        lemma my_lemma
    }
    spec module {
        lemma my_lemma()
    }
    spec module {
        lemma my_lemma() {}
    }
    spec module {
        lemma my_lemma<T, U>(a: u8) {
            pragma opaque;
        }

        lemma lemma_with_proof() {} proof { assume [trusted] true; }
    }
}
