module 0x1::m {
    enum S { One, Two }
}
module 0x1::main {
    fun main() {
        let s = 0x1::m::S::One;
                           //^ unresolved
    }
}        