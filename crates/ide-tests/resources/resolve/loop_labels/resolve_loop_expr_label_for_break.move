module 0x1::m {
    fun main() {
        'label: loop {
         //X
            break 'label;
                    //^
                   
        }
    }
}        