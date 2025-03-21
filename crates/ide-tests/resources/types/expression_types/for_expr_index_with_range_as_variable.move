module 0x1::m {
    fun main() {
        let vec = 1..10;
        for (i in vec) {
            i;
          //^ integer  
        }
    }
}        