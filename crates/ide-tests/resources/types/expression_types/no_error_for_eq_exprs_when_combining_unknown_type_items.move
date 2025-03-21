module 0x1::option {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    fun some<Element>(e: Element): Option<Element> { Option { vec: vector[e] } }
    fun main() {
        let unknown/*: unknown*/ = unknown_variable;
        let a2 = @0x1;
        unknown != some(a2);
        unknown == some(a2);
                   //^ 0x1::option::Option<address>
    }
}        