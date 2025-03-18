module 0x1::M {
    fun call() {}
    spec call {
        let count = 1;
           //X
        requires count > 1;
                //^
    }
}