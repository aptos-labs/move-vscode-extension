module 0x1::M {
    #[test_only]
    fun call() {}
       //X
    
    #[test]
    fun test_a() {
        call();
       //^
    }
}    