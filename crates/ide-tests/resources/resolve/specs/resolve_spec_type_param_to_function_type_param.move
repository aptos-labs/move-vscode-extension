module 0x1::M {
    fun m<CoinType>() {}
         //X
    spec m {
        ensures exists<CoinType>(@0x1);
                       //^
    }
}    