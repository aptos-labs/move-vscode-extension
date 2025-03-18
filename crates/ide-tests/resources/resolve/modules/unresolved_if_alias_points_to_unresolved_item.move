module 0x1::m {
    use 0x1::Transaction as MyTransaction;
                            //X
    fun main() {
        let a = MyTransaction::create();
              //^
    }
}