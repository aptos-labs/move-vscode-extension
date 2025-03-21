module 0x1::vector {
    /// Apply the function to each element in the vector, consuming it.
    public inline fun for_each<Element>(self: vector<Element>, _f: |Element|) {
    }
}
module 0x1::m {
    fun main() {
        let f = |m|;
        vector[1u8].for_each(f);
        f;
      //^ |u8| -> ()                
    }
}        