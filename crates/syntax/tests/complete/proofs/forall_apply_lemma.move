module 0x1::forall_apply_lemma {
    spec main {} proof {
        forall x: u64, y: u64 apply mylemma(x);
        forall x: u64, y: u64 { my_trigger(x) } apply mylemma(x);
    }
}
