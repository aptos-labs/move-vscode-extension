module 0x1::schema_includes_lit {
    spec module {
        include MySchema;
        include MySchema{ amount };
        include MySchema<MyType>{ amount };
        include MySchema{ address: Signer::address_of(acc) };
    }
}
