module 0x1::string {}
            //X

#[test_only]
module 0x1::string_tests {
    #[test]
    #[expected_failure(abort_code = 1, location = 0x1::string)]
                                                      //^
    fun test_abort() {
        
    }
}        