module 0x1::Original {
    friend 0x1::M;
    entry fun call() {}
}
module 0x1::M {
    use 0x1::Original;
    fun main() {
        Original::call();
                //^ unresolved
    }
}    