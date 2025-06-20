module 0x1::less_in_parens_with_complex_type {
    fun main() {
        while (i < 200) {
            assert!(table.contains(i), 0);
            assert!(table.remove(i) == i * 2, 0);
        }
    }

    fun main2() {
        let table: SmartTable<u64, u64> = new_with_config(1, 100, 2);
    }
}
