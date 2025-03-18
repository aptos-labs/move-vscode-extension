module 0x1::m {
    struct Coin { val: u8 }
          //X
}
module 0x1::main {
    use 0x1::m;
    spec module {
        let _ = m::Coin { val: 10 };
                   //^
    }
}