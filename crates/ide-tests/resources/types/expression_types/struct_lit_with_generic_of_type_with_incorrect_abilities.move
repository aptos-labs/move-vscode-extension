module 0x1::M {
    struct S<phantom Message: store> {}
    struct R has copy {  }
    fun main() {
        S<R> {};
      //^ 0x1::M::S<0x1::M::R>  
    }
}    