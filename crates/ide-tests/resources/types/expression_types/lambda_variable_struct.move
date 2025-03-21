module 0x1::m {
    struct S { val: u8 }
    fun call_on<Element>(i: Element, f: |Element|) {}
    fun main() {
        let select_val = |s| {
            s.val;
             //^ u8
        };
        let s = S { val: 1 };
        call_on(s, select_val);
    }
}        