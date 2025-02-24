module 0x1::control_exprs {
    fun main() {
        { true; true };
        loop { true; };
        while (true) { true; };
        for (i in 0..10) { true };
        if (true) { true } else { true };
        abort 1;
        return 1;
        return
    }
}
