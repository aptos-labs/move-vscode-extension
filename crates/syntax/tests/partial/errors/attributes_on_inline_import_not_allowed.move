module 0x1::attributes_on_inline_import_not_allowed {
    fun test_add(acc: signer) {
        #[test]
        use 0x1::M;
    }}
