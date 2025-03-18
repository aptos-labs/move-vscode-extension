module 0x1::M {
    fun call() {}
    spec call {
        spec_fun()
        //^
    }
    spec module {
        fun spec_fun() {}
           //X
    }
}    