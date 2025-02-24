module 0x1::schemas {
    spec schema MySchema {
        assert 1 == 1;
        one: path::MyType;
        local two: u8;
        local two_with_params: u8;
        global three: u8;
        global four<X, Y>: u8 = 1;
    }

    spec schema ModuleInvariant<X, Y> {
        requires global<X>(@0x0).f == global<X>(@0x1).f;
        ensures global<X>(@0x0).f == global<X>(@0x1).f;
    }
}
