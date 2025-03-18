module 0x1::m {
    public(friend) fun call() {}
}        
module 0x1::main {
    use 0x1::m::call;
    fun main() {
        call()
        //^ unresolved
    }
}