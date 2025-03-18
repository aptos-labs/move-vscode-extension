module 0x1::A {
    public fun create() {}
}
module 0x1::B {
    fun main() {
        let a = A::create();
              //^ unresolved
    }
}