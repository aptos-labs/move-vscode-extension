module 0x1::assert_macro {
    fun main() {
        assert!(true, 1);
        assert!(true);

        assert!((&Y<X<bool>>[addr]).field.value == false, 1);
        assert!(y_resource.field.value == false, 1);
    }
}
