// language=Move
pub const BUILTINS_FILE: &str = r#"
    module 0x0::builtins {
        const MAX_U8: u8 = 255;
        const MAX_U16: u16 = 65535;
        const MAX_U32: u32 = 4294967295;
        const MAX_U64: u64 = 18446744073709551615;
        const MAX_U128: u128 = 340282366920938463463374607431768211455;
        const MAX_U256: u256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;

        /// Removes `T` from address and returns it.
        /// Aborts if address does not hold a `T`.
        native fun move_from<T: key>(addr: address): T acquires T;

        /// Publishes `T` under `signer.address`.
        /// Aborts if `signer.address` already holds a `T`.
        native fun move_to<T: key>(acc: &signer, res: T);

        native fun borrow_global<T: key>(addr: address): &T acquires T;

        native fun borrow_global_mut<T: key>(addr: address): &mut T acquires T;

        /// Returns `true` if a `T` is stored under address
        native fun exists<T: key>(addr: address): bool;

        native fun freeze<S>(mut_ref: &mut S): &S;

        spec native fun max_u8(): num;
        spec native fun max_u64(): num;
        spec native fun max_u128(): num;
        spec native fun global<T: key>(addr: address): T;
        spec native fun old<T>(t: T): T;
        spec native fun update_field<S, F, V>(s: S, fname: F, val: V): S;
        spec native fun TRACE<T>(t: T): T;

        spec native fun concat<T>(v1: vector<T>, v2: vector<T>): vector<T>;
        spec native fun vec<T>(t: T): vector<T>;
        spec native fun len<T>(t: vector<T>): num;
        spec native fun contains<T>(v: vector<T>, e: T): bool;
        spec native fun index_of<T>(v: vector<T>, e: T): num;
        spec native fun range<T>(v: vector<T>): range;
        spec native fun update<T>(v: vector<T>, i: num, t: T): vector<T>;
        spec native fun in_range<T>(v: vector<T>, i: num): bool;

        spec native fun int2bv(i: num): bv;
        spec native fun bv2int(b: bv): num;
    }
"#;
