module 0x1::main {
    fun call<T>(a: T, b: T): T {
        b        
    }    
    fun main() {
        let aa = call<u8>(1u64, 1u128);
        aa;
      //^ u8  
    }    
}        