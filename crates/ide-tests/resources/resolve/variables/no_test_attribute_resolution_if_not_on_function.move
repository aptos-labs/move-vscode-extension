module 0x1::M {
    fun test_add(acc: signer) {
        #[test(acc = @0x1)]
              //^ unresolved
        use 0x1::M;            
    }
}    