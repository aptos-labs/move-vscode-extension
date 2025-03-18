module std::mymodule {
           //X
    public fun call() {}
}
module 0x1::main {
    fun main() {
        std::mymodule::call();
             //^
    }
}         