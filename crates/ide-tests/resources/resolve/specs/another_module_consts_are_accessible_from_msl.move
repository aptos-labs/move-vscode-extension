module 0x1::M {
    const MY_CONST: u8 = 1;
          //X
}    
module 0x1::M2 {
    use 0x1::M;
    spec module {
        M::MY_CONST;
             //^            
    }
}