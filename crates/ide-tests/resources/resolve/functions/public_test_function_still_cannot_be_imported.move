module 0x1::m1 {
    #[test]
    public fun test_a() {}
}  
module 0x1::m2 {
    use 0x1::m1::test_a;
               //^ unresolved
}    