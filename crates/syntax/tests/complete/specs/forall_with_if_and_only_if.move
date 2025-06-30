module 0x1::forall_with_if_and_only_if {
    spec module {
        ensures [abstract] forall k: K: vector::spec_contains(keys,k) <==> spec_contains_key(result, k);
    }
}
