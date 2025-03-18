address 0x1 {
    module Transaction {
        struct Sender {}
             //X
    }
}
script {
    use 0x1::Transaction::Sender;

    fun main(n: Sender) {}
              //^
}