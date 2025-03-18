module 0x1::coin {
}
spec 0x1::coin {
    spec module {
        global supply<CoinType>: num;
              //X
    }
}        
module 0x1::transaction {
    fun main() {}
}
spec 0x1::transaction {
    spec main {
        use 0x1::coin;
        ensures coin::supply<CoinType> == 1;
                        //^
    }
}