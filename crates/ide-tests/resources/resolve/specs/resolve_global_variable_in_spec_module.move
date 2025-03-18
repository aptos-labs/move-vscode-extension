module 0x1::m {}
spec 0x1::m {
    spec module {
        global supply<CoinType>: num;
               //X
    }
    spec schema MySchema {
        ensures supply<CoinType> == 1;
                  //^    
    }
}        