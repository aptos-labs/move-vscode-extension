module 0x1::assert_macro {
    fun main() {
        assert!(true, 1);
        assert!(true);

        assert!((&Y<X<bool>>[addr]).field.value == false, 1);
        assert!(y_resource.field.value == false, 1);

        assert!(true, b"1234 {}");
        assert!(true, b"1234 {}", 1);
        assert!(true, b"1234 {}", 1, 2);
        assert!(true, b"1234 {}", 1, 2, 3);
        assert!(true, b"1234 {}", 1, 2, 3, 4);

        assert_eq!(1, 1);
        assert_ne!(1, 1);

        assert_eq!(1, 1, b"1234");
        assert_eq!(1, 1, b"1234", 1, 2, 3, 4);
    }
}
