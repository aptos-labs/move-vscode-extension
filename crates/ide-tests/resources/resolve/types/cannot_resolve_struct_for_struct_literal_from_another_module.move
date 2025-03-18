module 0x1::s {
    struct MyStruct {}
}
module 0x1::m {
    use 0x1::s::MyStruct;
    fun call() {
        let a = MyStruct {};
              //^ unresolved
    }
}