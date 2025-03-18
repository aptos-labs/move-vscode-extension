address 0x1 {
    module Transaction {
         //X
        fun create() {}
    }
    
    module M {
        use 0x1::Transaction::{Self};
        fun main() {
            let a = Transaction::create();
                  //^
        }
    }
}        