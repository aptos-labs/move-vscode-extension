module 0x1::m {
          //X
    fun create() {}
}         
module 0x1::main {
    use 0x1::m::{Self};
    fun main() {
        let a = m::create();
              //^
    }
}