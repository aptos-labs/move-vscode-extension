module 0x1::m {
    public inline fun for_each<Element>(o: Element, f: |Element|) {}
    fun main() {
        for_each(1, |value|
                     //X
            value
            //^
        )
    }
}        