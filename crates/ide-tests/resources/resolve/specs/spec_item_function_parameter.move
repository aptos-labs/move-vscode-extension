module 0x1::main {
    fun call(account: &signer) {}
             //X
}        
spec 0x1::main {
    spec call(account: &signer) {
                //^
    
    }
}