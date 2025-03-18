address 0x0002 {
    module A {
         //X
    }
}

address 0x1 {
    module B {
        use 0x02::A;
        
        fun main() {
            let a = A::create();
                  //^
        }
    }
}