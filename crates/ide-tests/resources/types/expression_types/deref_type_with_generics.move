module 0x1::m {
    struct Coin<CoinType> { val: u8 }
    struct BTC {}
    fun main() {
        let a = &mut Coin { val: 10 };
        let b: Coin<BTC> = *a;
        a;
      //^ &mut 0x1::m::Coin<0x1::m::BTC>  
    }        
} 