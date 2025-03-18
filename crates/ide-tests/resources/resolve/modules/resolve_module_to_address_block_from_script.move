address 0x2 {
    module A {
         //X
    }
}

script {
    use 0x2::A;
    
    fun main() {
        let a = A::create();
              //^
    }
}