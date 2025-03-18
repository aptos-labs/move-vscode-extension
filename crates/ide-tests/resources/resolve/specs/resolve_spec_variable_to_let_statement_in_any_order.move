module 0x1::M {
    fun call() {}
    spec call {
        requires count > 1;
                //^
        let count = 1;
           //X
    }
}