module 0x1::m {
    use 0x1::Transaction::Sender as MySender;
    fun main(n: MySender) {}
              //^ unresolved
}