module 0x1::m {
    fun call<Element>(i: Element, f: |vector<Element>|) {}
    fun main() {
        let f = |amounts| {
            let amount = amounts[0];
            amount;
            //^ u8
        };
        call(1u8, f);
        
    }
}        