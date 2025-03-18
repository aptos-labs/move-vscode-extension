module 0x1::caller {
    public fun call() {}
              //X
}
module 0x1::main {
    public fun main() {
        call();
        //^
    }
}    
spec 0x1::main {
    use 0x1::caller::call;
}