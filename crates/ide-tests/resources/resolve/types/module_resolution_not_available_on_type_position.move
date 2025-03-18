module 0x1::Transaction {
    struct Type {
        val: u8                   
    }
}
module 0x1::M {
    fun main(a: 0x1::Transaction::Transaction) {
                                   //^ unresolved
    }
}