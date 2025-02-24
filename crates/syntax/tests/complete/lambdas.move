module 0x1::lambdas {
    fun main(l: |u8| u8, m: |u8|) {
        for_each(v, |f: &Function| {});
        for_each(v, |i: u8, g: u8| {});
    }
}
