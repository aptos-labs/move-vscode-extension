module 0x1::M {
    spec fun call<TypeParam>() {
                    //X
        spec_exists<TypeParam>();
                   //^
    }
    spec fun spec_exists<TypeParam>() { true }
}    