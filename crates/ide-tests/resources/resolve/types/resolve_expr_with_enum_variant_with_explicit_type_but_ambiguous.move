module 0x1::m {
    enum S { One, Two }
           //X
    enum T { One, Two }
    fun main() {
        let a: S = One;
                  //^
    }
}        