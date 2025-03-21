module 0x1::option {
    fun some<Element>(e: Element): Element { e }
    fun main() {
        let unknown/*: unknown*/ = unknown_variable;
        let a2 = 1;
        some(a2) == unknown;
        //^ integer
    }
}        