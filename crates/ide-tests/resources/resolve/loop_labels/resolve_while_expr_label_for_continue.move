module 0x1::m {
    fun main() {
        'label: while (true) {
         //X
            continue 'label;
                      //^
                   
        }
    }
}        