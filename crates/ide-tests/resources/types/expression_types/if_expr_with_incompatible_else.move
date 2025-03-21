module 0x1::M {
    fun m() {
        let a = if (true) 1 else true;
        a;
      //^ <unknown>
    }
}    