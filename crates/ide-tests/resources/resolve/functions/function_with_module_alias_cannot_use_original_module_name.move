module 0x1::m {
    public fun call() {}
}

module 0x1::main {
    use 0x1::m as m_alias;
    
    fun main() {
        m::call();
         //^ unresolved
    }
}    