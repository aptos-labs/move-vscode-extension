module 0x1::signer {
          //X
    fun address_of(addr: address) {}
}     
module 0x1::Module {
    use 0x1::signer;
}    
spec 0x1::Module {
    spec schema MySchema {
        let a = signer::;  
               //^
    }
}