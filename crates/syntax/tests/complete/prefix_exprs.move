module 0x1::prefix_exprs {
    fun main() {
        move 1;
        copy 1;
        *1;
        !1;
        move copy !*1;
        1
    }
}
