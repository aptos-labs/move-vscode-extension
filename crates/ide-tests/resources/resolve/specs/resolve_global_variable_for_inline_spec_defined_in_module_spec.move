module 0x1::m {
    fun main() {
        spec {
            assert supply<CoinType> == 1;
                      //^    
        }
    }
}        
spec 0x1::m {
    spec module {
        global supply<CoinType>: num;
               //X
    }
}