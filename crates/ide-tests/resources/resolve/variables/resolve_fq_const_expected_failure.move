module 0x1::string {
    const ERR_ADMIN: u64 = 1;
          //X
}        
#[test_only]
module 0x1::string_tests {
    #[test]
    #[expected_failure(abort_code = 0x1::string::ERR_ADMIN)]
                                                //^
    fun test_abort() {
        
    }
}