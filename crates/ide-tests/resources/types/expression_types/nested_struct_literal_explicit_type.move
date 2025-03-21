module 0x1::main {
    struct V<T> { val: T }
    struct S<T> { val: V<T> }
    fun main() {
        let s = S { val: V<u64> { val: }};
        s;
      //^ 0x1::main::S<u64>  
    }
}        