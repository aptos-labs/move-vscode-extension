module 0x1::function_values_struct_wrapper {
    struct Predicate<T>(|&T|bool) has copy;
    fun main() {
        let f: Predicate<u64> = |x| *x > 0;
        assert!(f(&22));
    }
}
