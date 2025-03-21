module 0x1::m {
    struct Table { length: u64 }
}        
spec 0x1::m {
    spec Table {
        pragma map_length = length;
                            //^ num
    }
}