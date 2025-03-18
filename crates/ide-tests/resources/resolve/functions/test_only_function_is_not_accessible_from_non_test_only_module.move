module 0x1::M1 {
    #[test_only]
    public fun call() {}
}        
module 0x1::M2 {
    use 0x1::M1;
    fun call() {
        M1::call();
           //^ unresolved             
    }
}