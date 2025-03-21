module 0x1::M {
    fun call(): u8 { 1 }
    spec module {
        call();
        //^ num
    }
}    