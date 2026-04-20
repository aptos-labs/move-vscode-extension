module 0x1::forall_quant_allowed_in_spec_fun {
    spec fun all_positive(): bool {
        forall i in 0..10: i >= 0
    }
}
