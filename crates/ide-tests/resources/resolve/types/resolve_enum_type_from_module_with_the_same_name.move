module 0x1::S { 
    enum S { One, Two }
       //X
}
module 0x1::main {
    use 0x1::S;
    fun main(one: S::S) {
                   //^
    }
}        