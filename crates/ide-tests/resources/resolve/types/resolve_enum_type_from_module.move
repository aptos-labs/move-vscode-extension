module 0x1::m { 
    enum S { One, Two }
       //X
}
module 0x1::main {
    use 0x1::m;
    fun main(one: m::S) {
                   //^
    }
}        