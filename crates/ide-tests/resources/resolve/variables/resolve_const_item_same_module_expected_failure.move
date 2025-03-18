#[test_only]
module 0x1::string_tests {
    const ERR_ADMIN: u64 = 1;
        //X
    
    #[test]
    #[expected_failure(abort_code = ERR_ADMIN)]
                                     //^
    fun test_abort() {
        
    }
}