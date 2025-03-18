module 0x1::Transaction {
    struct Sender { val: u8 }
}
module 0x1::m {
    use 0x1::Transaction::Sender as MySender;
                                  //X
    fun main(): MySender {}
              //^
}