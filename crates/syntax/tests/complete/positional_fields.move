module 0x1::positional_fields {
    struct S1(u8);
    struct S2<T>(u8, bool, aptos_framework::option::Option<T>);

    struct S01() has copy;
    struct S02 { val: u8 } has copy;

    enum E<T> {
        V(bool, aptos_framework::option::Option<T>)
    }

    fun construct() {
        let x = S(42);
        let y = E::V(true, 42);
    }

    fun destruct(x: S, y: E) {
        x.0;
        let S(_x) = x;
        let E::V(_x, _y) = x;
    }
}
