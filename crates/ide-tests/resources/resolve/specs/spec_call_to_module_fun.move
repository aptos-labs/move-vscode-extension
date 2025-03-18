module 0x1::M {
    fun mod_exists(): bool { true }
       //X
    fun call() {}
    spec call {
        ensures mod_exists();
                //^
    }
}    