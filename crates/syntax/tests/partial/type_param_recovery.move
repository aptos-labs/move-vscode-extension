module 0x1::type_param_recovery {
    struct S1<T,,>();
    struct S2<T:>();
    struct S3<T:,>();
    struct S4<T:copy+,>();

    struct S5<phantom>();
    struct S6<phantom,>();
    struct S7<D:1>();

    fun main<a:, phantom,, 11, bb:copy+, dd: 1>() {

    }
}
