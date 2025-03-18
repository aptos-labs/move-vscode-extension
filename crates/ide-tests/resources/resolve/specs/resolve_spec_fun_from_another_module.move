module 0x1::M {
    spec fun myfun(): bool { true }
           //X
}
module 0x1::M2 {
    use 0x1::M;
    spec module {
        M::myfun();
          //^
    }
}