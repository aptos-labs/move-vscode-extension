module 0x1::schema_includes_lit {
    spec module {
        include if (true) MySchema else MySchema;
        include MySchema && MySchema;
    }
}
