module 0x1::schema_includes_imply_exists {
    spec module {
        include add_all_currencies && !exists<Balance<XUS>>(addr)
            ==> Diem::AbortsIfNoCurrency<XUS>;
    }
}
