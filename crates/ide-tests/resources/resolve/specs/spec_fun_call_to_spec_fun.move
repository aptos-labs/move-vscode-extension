module 0x1::M {
    spec fun call() {
        spec_exists();
            //^
    }
    spec fun spec_exists() { true }
            //X
}    