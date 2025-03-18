module 0x1::m {
    enum S { One(u8), Two }
                     //X
    fun main() {
        S::Two(1);
          //^
    }
}        