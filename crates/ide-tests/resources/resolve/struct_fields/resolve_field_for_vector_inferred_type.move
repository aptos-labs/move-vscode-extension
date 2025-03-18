module 0x1::M {
    struct ValidatorInfo { field: u8 }
                          //X
    native public fun vector_empty<El>(): vector<El>;
    native public fun vector_push_back<PushElement>(v: &mut vector<PushElement>, e: PushElement);
    native public fun vector_borrow_mut<BorrowElement>(v: &mut vector<BorrowElement>, i: u64): &mut BorrowElement;
    fun call() {
        let v = vector_empty();
        let item = ValidatorInfo { field: 10 };
        vector_push_back(&mut v, item);
        vector_borrow_mut(&mut v, 10).field;
                                      //^
    }
}        