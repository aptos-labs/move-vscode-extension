module 0x2::A {}
          //X
module 0x1::B {
    use 0x2::A;
    
    fun main() {
        let a = A::create();
              //^
    }
}