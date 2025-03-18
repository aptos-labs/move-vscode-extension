module 0x1::features {
    const PERMISSIONED_SIGNER: u64 = 84;
           //X
    
}
module 0x1::m {}
spec 0x1::m {
    spec fun is_permissioned_signer(): bool {
        use 0x1::features::PERMISSIONED_SIGNER;
        PERMISSIONED_SIGNER;
        //^
    }
}    