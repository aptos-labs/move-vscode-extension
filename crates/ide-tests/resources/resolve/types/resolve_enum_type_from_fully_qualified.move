module 0x1::m { 
    enum S { One, Two }
       //X
}
module 0x1::main {
    fun main(one: 0x1::m::S) {
                        //^
    }
}        