module 0x1::M {
    fun main() {
        let a = while (true) { 1; };
        a;
      //^ <never>  
    }
}    