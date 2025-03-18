module 0x1::m {
    #[test(unknown_attr(my_signer = @0x1))]
                         //^ unresolved
    fun test_main(my_signer: signer) {
    }
}        