module 0x1::M {
    struct CapState<phantom Feature> has key { delegates: vector<address> }
                                                 //X
    fun m() acquires CapState {
        borrow_global_mut<CapState<u8>>(@0x1).delegates;
                                               //^
    }
}    