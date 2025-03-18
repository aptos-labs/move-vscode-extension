module 0x1::M1 {
    struct S { val: u8 }
    public fun get_s(): S { S { val: 10 } }
}        
module 0x1::M {
    use 0x1::M1;
    fun main() {
        M1::get_s().val
                   //^ unresolved
    }            
} 