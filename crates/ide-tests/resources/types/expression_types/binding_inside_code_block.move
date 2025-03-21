module 0x1::main {
    fun main() {
        let b = {
            let in = 1;
            in;
          //^ integer  
        };
    }
}        