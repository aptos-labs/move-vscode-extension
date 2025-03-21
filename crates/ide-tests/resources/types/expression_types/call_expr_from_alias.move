module 0x1::string {
    public fun call(): u8 { 1 }
}        
module 0x1::main {
    use 0x1::string::call as mycall;
    fun main() {
        let a = mycall();
        a;
      //^ u8  
    }
}