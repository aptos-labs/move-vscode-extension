module 0x1::m {
    enum S1 { One, Two }
             //X  
    enum S2 { One, Two }
    fun main(_: S1) {
        let s: S1 = One;
                   //^
    }
}        