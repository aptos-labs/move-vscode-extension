module 0x1::m {
    spec module {
        global supply<CoinType>: num;
    }
    fun main() {
        let supply = 1;
            //X
        spec {
            supply;
            //^ 
        }
    }
}        