module 0x1::royalty {}
module 0x1::m {
    use 0x1::royalty::Self;
    public fun royalty() {}
                //X
    public fun main() {
        royalty();
        //^
    }
}        