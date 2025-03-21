module 0x2::A {
          //X
}
module 0x1::B {
    fun main() {
        let a = 0x2::A::create();
                   //^
    }
}