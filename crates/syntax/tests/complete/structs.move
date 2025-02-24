module 0x1::structs {
    struct S1<T> { val: T }
    struct S2<phantom T> { val: u8 }
    struct S3(u8);
    struct S4<T>(T, T);
}
