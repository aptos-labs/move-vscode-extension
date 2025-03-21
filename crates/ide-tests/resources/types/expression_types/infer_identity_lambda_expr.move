module 0x1::m {
    inline fun for_each<Element>(v: vector<Element>, f: |Element| Element) {}
    fun main() {
        for_each(vector[1, 2, 3], |elem| elem);
                                          //^ integer
    }
}