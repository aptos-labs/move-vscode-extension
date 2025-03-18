module 0x1::signer {
          //X
    fun address_of(addr: address) {}
}     
module 0x1::mod {
}    
spec 0x1::mod {
    spec schema MySchema {
        use 0x1::signer;
        let a = signer::;  
               //^
    }
}