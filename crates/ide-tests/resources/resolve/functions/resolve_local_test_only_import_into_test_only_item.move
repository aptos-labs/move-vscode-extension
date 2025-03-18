module 0x1::m {
    #[test_only]
    public fun call() {}
              //X
}        
module 0x1::main {
    #[test]                        
    fun main() {
    use 0x1::m::call;
                //^
    }
}