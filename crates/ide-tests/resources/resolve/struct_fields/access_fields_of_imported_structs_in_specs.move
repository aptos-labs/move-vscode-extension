module 0x1::coin {
    struct Coin { value: u64 }
                 //X
    public fun get_coin(): Coin { Coin { value: 10 } }
}        
module 0x1::m {
    use 0x1::coin::get_coin;
    
    spec module {
        get_coin().value;
                     //^
    } 
}