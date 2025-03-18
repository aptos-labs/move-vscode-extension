address 0x2 {
    module A {
         //X
    }
}

address 0x1 {
    module B {
        fun main() {
            let a = 0x2::A::create();
                       //^
        }
    }
}