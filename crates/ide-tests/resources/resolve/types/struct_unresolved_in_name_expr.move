address 0x1 {
    module A {
        struct S {}
    }
    module B {
        use 0x1::A;
        fun call() {
            A::S
             //^ unresolved                   
        }
    }
}