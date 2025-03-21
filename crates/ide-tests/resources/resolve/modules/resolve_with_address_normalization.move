module 0x0002::A {
             //X
}
module 0x1::B {
    use 0x02::A;
    
    fun main() {
        let a = A::create();
              //^
    }
}