module 0x1::prefer_stmt_block_no_call {
    fun if_then_parens() {
        if (true) { 1 } else { 2 }
        (1 + 2);
    }

    fun loop_then_parens() {
        loop { break }
        (1);
    }
}