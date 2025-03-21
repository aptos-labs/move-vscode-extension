module 0x1::m {}        
spec 0x1::m {
    spec Table {
        let left_length = 100;
        let left = vector[];
        let right = vector[];
        ensures forall i: u64 where i < left_length: left[i] == right[i];
                                  //^ num
    }
}