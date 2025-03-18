#[test_only] 
module 0x1::M1 {
    public fun call() {}
              //X
}   
#[test_only] 
module 0x1::M2 {
    use 0x1::M1::call;
    
    fun main() {
        call();
        //^
    }
}