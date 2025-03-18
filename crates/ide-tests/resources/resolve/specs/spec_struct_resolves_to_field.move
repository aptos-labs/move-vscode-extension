module 0x1::M {
    struct S { val: u8 }
              //X
    spec S {
        invariant val > 1;
                 //^ 
    }
}    