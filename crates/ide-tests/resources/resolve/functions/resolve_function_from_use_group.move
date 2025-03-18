module 0x1::m {
    public fun call() {}
              //X
}        
module 0x1::main {
    use 0x1::m::{call};
    public fun main() {
        call();
        //^
    }
}