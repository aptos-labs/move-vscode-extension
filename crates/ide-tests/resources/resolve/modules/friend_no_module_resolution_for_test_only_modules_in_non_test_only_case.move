module 0x1::M {
    friend 0x1::MTest;
               //^ unresolved
}    
#[test_only]
module 0x1::MTest {}