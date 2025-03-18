module 0x1::original {
    fun call() {}
}
module 0x1::m {
    use 0x1::original::call;
                     //^ unresolved
}