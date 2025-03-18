module 0x1::transaction {}
module 0x1::m {
    fun main() {
        let a = 0x3::transaction::create();
                     //^ unresolved
    }
}