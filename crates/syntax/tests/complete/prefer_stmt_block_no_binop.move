module 0x1::prefer_stmt_block_no_binop {
    fun if_then_binop() {
        if (true) { 1 } else { 2 }
        1 + 2;
    }

    fun while_then_binop() {
        while (true) { }
        1 + 2;
    }

    fun loop_then_binop() {
        loop { break }
        1 + 2;
    }
}