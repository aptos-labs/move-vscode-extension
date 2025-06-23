module 0x1::call_expr_complex_inequality {
    fun main() {
        take_first(
            if (a_less) { a + b } else { a_clone - neg_b },
            if (!a_less) { a_clone - neg_b } else { a + b }
        );
    }
}
