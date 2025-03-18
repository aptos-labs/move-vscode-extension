module 0x1::m {
    struct Coin<Token> {}
    
    native fun main<Token>()
                  //X
            : Coin<Token>;
                 //^
}