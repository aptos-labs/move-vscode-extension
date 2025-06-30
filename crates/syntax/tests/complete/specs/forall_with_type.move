module 0x1::forall_with_type {
    spec module {
        ensures [abstract] forall k: K: vector::spec_contains(keys,k);
    }
}
