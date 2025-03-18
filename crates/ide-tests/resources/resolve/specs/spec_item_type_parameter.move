module 0x1::main {
    fun call<CoinType>() {}
             //X
}        
spec 0x1::main {
    spec call<CoinType>() {
                //^
    
    }
}