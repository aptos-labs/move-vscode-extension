module aptos_std::m1 {
    struct Type { val: u8 }
          //X
}
module 0x1::m {
    use aptos_std::m1;
    use aptos_std::m1;
    fun main(s: m1::Type) {}
                   //^
}        