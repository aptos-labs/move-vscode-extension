script {
    fun main() {
        let a;
        let (a, b);
        let R { a, b };
        let R { a: _, b: _ };

        let R { a: alias_a, b: alias_b };
        let R { a: T { c, d }};
        let R { a: T { c: alias_c, d: _ }};

        let (R { a, b }, M { a: _, b: _ });

        let Generic<R> {};
        let Generic<R> { g };
        let Generic<R> { g: R { f: f3 } };

        let a: (u8);
        let a: (((u8)));
        let b: ((u8, u8));
    }
}
