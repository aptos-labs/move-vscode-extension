module 0x1::M {
    struct S<K, V> { val: u8 }
                   //X
    fun get_s<K, V>(): S<K, V> { S<u8, u8> { val: 10} }
    #[test]
    fun test_s() {
        let s = get_s();
        s.val;
         //^
    }
} 