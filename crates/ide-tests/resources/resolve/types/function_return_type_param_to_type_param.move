module 0x1::m {
    struct Coin<Token> {}
    
    fun main<Token>()
           //X
            : Coin<Token> {}
                 //^
}