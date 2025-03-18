module std::mymodule {
    public fun call() {}
              //X
}
module 0x1::main {
    fun main() {
        std::mymodule::call();
                       //^
    }
}         