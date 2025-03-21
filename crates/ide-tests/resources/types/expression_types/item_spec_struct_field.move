module 0x1::m {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    struct S { aggregator: Option<u8> }
    spec S {
        aggregator;
        //^ 0x1::m::Option<num>
    }
}        