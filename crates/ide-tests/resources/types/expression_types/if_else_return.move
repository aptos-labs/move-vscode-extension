module 0x1::M {
    fun main() {
        if (true) { return 1 } else { return 2 };
      //^ <never>  
    }
}    