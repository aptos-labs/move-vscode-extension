// line
script {
/* block */
/**
multiline
*/
    /** /* nest */ // lin */
    fun main() {}
// /* unclosed block
// /* block comment */
}

/// doc comment
/// another doc comment
module 0x1::M {
    /// function doc comment
    fun m() {}
    /// doc comment attr
    #[test_only]
    fun main() {
        let _ = /*caret*/1;
    }

    /// docs
    native fun native_m();

    /// docs
    struct S1 {}
    /// docs
    struct S2(u8);

    /// docs
    enum S {
        /// docs
        One,
        /// docs
        Two
    }
}

/* unclosed