module 0x1::minter {
    struct S {}
    public fun mint() {}    
}        
module 0x1::main {
    #[verify_only]
    use 0x1::minter::{Self, mint};
    
    public fun main() {
        mint();
        //^ unresolved 
    }
}          