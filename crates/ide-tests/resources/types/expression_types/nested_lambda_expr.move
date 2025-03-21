module 0x1::m {
    fun call<Element>(g: || |Element| Element) {}
    fun main() {
        let g = || { let f = |m| m; f };
        call(|| { |m: u8| m });
                        //^ u8  
    }
}        