module 0x1::M {
    fun m() {}
    spec m {
        fun call() {}
           //X
        call();
        //^
    }
}    