#[test_only]
module 0x1::attributes_on_items {
    #[test_only]
    use 0x1::m;
    #[test_only]
    use 0x1::m::call;
    #[test_only]
    fun main() {}
    #[test_only]
    const S: u8 = 1;
    #[test_only]
    friend 0x1::m;
    #[test_only]
    inline fun main() {}
    #[test_only]
    struct S {
        val: u8
    }
    #[test_only]
    enum S {
        #[test_only]
        One {
        },
        Two
    }
    #[test_only]
    spec fun main(): u8 { 1 }
    #[test_only]
    spec module {
        fun main(): u8 { 1 }
    }
    #[test_only]
    spec main {}
    #[test_only]
    spec schema MySchema {
    }
}
