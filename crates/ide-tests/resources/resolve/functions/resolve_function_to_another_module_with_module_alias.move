module 0x1::m {
    public fun call() {}
             //X
}

module 0x1::main {
    use 0x1::m as m_alias;
    
    fun main() {
        m_alias::call();
                //^
    }
}    