module 0x1::original {
    friend 0x1::m;
    public(friend) fun call() {}
}
module 0x1::m {}
script { 
    use 0x1::original;
    fun main() {
        original::call();
                //^ unresolved
    }
}