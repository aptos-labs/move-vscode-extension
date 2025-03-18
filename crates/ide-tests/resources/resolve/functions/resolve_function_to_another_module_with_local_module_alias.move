module 0x1::m {
    public fun call() {}
             //X
}

module 0x1::main {
    fun main() {
        use 0x1::m as m_alias;
        m_alias::call();
                //^
    }
}    