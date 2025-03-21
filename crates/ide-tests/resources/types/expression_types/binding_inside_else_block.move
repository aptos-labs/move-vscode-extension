module 0x1::main {
    fun main() {
        if (true) {} else {
            let in = 1;
            in;
          //^ integer  
        }
    }
}        