module 0x1::M {
    struct Option<Element> {}
    fun call() {
        let a = Option<u8> {};
        a;
      //^ 0x1::M::Option<u8>  
    }
}        