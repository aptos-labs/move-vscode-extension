module 0x1::m {
}        
spec 0x1::m {
    spec module {
        fun spec_sip_hash();
            //X
    }
}
module 0x1::main {
    use 0x1::m::spec_sip_hash;
               //^
}