module 0x2::A {}
          //X
script {
    use 0x2::A;
    
    fun main() {
        let a = A::create();
              //^
    }
}