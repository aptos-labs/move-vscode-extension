module aptos_std::m1 {
    struct Type { val: u8 }
          //X
}
module 0x1::m {
    use aptos_std::m1::Type;
    use aptos_std::m1::Type;
    fun main(s: Type) {}
               //^
}        