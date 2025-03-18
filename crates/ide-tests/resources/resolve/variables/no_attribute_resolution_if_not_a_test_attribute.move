module 0x1::M {
    #[test]
    #[expected_failure(abort_code = 1)]
                         //^ unresolved
    fun call(abort_code: signer) {
        
    }
}    