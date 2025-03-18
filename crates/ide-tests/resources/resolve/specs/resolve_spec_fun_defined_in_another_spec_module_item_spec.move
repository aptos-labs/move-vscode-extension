module 0x1::m {
}
spec 0x1::m {
    spec module {
        fun spec_now_microseconds(): u64 {
                  //X
            1
        }            
    }
}
module 0x1::main {
    use 0x1::m;
    spec module {
        m::spec_now_microseconds();
             //^
    }
}