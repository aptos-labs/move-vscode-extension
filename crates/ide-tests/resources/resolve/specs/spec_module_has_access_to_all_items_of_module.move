module 0x1::Module {
    fun address_of(addr: address) {}
       //X
}    
spec 0x1::Module {
    spec schema MySchema {
        let a = address_of(@0x1);  
                //^
    }
}