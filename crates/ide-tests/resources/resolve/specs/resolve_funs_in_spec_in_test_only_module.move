#[test_only]
module 0x1::m {
    public fun simple_share(o: Obj) {
               //X
    }
    spec fun call(): u128 {
        simple_share(); 1
        //^
    }

}