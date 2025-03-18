module 0x1::m { 
    enum S { One, Two }
}
module 0x1::main {
    use 0x1::m;
    fun main() {
        let a = m::S::One;
                     //^ unresolved
    }
}        