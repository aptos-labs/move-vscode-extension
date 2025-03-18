module aptos_std::m1 {
    struct Type { val: u8 }
          //X
}
module aptos_framework::m2 {
}
module 0x1::m {
    use aptos_framework::m1::Type;
    fun main(s: Type) {}
               //^
}        