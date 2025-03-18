address 0x1 {
    module A {
         //X
    }

    module B {
        use 0x1::A;
               //^
    }
}