module 0x1::M {
    fun call() {}
    spec call {
        let count = 1;
           //X
        let count2 = count + 1;
                   //^
    }
}