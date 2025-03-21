module 0x1::vector {
    public fun enumerate_ref<Element>(self: vector<Element>, _f: |&Element| Element) {}
}
module 0x1::m {
    fun main() {
        let f = |to| (*to);
                      //^ &address
        vector[@0x1].enumerate_ref(f);
    }
}     