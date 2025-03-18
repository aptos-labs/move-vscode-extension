module 0x1::m {
    spec module {
        global supply: num;
    }
    fun main() {
        supply;
        //^ unresolved
    }
}        