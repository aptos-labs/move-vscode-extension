module 0x1::spec_block_with_lambdas {
    fun apply_with_no_abort(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    fun test_no_abort(): u64 {
        apply_with_no_abort(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
            5  // 5 != MAX_U64, so !aborts_if is satisfied
        )
    }
}
