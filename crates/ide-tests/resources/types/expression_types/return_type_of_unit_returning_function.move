module 0x1::M {
    fun call(): () {}
    fun m() {
        call();
      //^ ()
    }
}    