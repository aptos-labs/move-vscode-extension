module 0x1::M {
    fun call() {}
    spec call {
        ensures spec_exists();
                //^
    }
    spec fun spec_exists() { true }
            //X
}    