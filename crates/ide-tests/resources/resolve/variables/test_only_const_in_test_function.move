module 0x1::M {
    #[test_only]
    const TEST_CONST: u64 = 1;
          //X
    #[test]
    fun test_a() {
        TEST_CONST;
            //^
    }
}    