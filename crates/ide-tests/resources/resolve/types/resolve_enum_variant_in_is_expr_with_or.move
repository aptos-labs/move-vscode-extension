module 0x1::m {
    enum S1 { One, Two }
                  //X  
    enum S2 { One, Two }
    fun main(s: S1) {
        s is One | Two;
                  //^
    }
}        