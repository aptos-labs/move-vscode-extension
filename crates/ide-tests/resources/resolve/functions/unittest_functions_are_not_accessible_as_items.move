#[test_only]    
module 0x1::M {
    #[test]
    fun test_a() {}
    fun main() {
        test_a();
       //^ unresolved 
    }
}    