module 0x1::m {
    fun main() {
        for (i in false..true) {
            i;
          //^ bool  
        };
    }
}        