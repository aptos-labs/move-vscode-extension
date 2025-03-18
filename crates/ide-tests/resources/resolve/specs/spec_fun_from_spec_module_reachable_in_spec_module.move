module 0x1::M {
    spec module {
        fun call() {}
           //X
        fun m() {
            call();
            //^
        }
    }
}    