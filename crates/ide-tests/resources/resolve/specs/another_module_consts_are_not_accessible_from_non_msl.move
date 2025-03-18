module 0x1::M {
    const MY_CONST: u8 = 1;
}    
module 0x1::M2 {
    use 0x1::M;
    fun main() {
        M::MY_CONST;
             //^ unresolved            
    }
}