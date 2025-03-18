module 0x1::M {
    #[test_only]
    friend 0x1::MTest;
               //^
}    
#[test_only]
module 0x1::MTest {}
           //X