module 0x1::m {
    struct Coin has key {}
    spec module {
        modifies (global<Coin>(@0x1));
               //^ 0x1::m::Coin
    }
}        