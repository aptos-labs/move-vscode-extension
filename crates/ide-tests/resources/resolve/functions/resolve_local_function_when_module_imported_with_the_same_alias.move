module 0x1::myroyalty {}
module 0x1::m {
    use 0x1::myroyalty as royalty;
    public fun royalty() {}
                //X
    public fun main() {
        royalty();
        //^
    }
}        