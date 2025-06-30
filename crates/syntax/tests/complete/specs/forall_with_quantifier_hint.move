module 0x1::forall_with_quantifier_hint {
    spec module {
        ensures [abstract] forall k: K {spec_contains_key(result, k)} : vector::spec_contains(keys,k) <==> spec_contains_key(result, k);
        ensures forall k: K {
            spec_contains_key(self, k),
            std::cmp::compare(option::spec_borrow(result), k),
            std::cmp::compare(key, k)
        }: true;
    }
}
