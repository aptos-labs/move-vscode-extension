#[test_only]    
module 0x1::M1 {
    #[test]
    entry fun test_a() {}
}    
#[test_only]
module 0x1::M2 {
    use 0x1::M1; 
    
    entry fun main() {
        M1::test_a();
           //^ unresolved    
    }
}    