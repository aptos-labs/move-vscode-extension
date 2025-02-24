module 0x1::m {
    friend 0x1::M2;

    #[test_only]
    friend 0x2::M2;
}
