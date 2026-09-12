module 0x1::behavior_predicate_no_arg_list {
    fun f(g: |u64| u64, a: u64, b: u64) {}
    spec f {
        aborts_if aborts_of<self.settle_trade_f>(a, b);
        aborts_if aborts_of<g>;
    }
}
