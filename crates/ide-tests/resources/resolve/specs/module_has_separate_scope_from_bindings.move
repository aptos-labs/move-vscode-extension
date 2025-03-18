module 0x1::m {
    fun call(account: &signer) {}
                //X
    spec call {
        use aptos_framework::account;
        signer::address_of(account);
                          //^
    }
}        