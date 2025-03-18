module 0x1::m {
    struct S { val: u8 }
              //X
    fun call_on<Element>(i: Element, f: |Element|) {}
    fun main() {
        let select_val = |s| {
            s.val;
             //^
        };
        let t = S { val: 1 };
        call_on(t, select_val);
    }
}        