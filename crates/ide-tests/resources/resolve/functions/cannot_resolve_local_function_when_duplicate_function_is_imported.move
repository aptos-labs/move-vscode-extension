module 0x1::royalty {
    public fun royalty() {}
}
module 0x1::m {
    use 0x1::royalty::royalty;
    public fun royalty() {}
    public fun main() {
        royalty();
        //^ unresolved
    }
}        