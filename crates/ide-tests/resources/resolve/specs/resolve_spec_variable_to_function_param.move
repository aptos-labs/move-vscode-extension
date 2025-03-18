module 0x1::M {
    fun call(count: u8) {}
            //X
    spec call {
        requires count > 1;
                //^
    }
}    