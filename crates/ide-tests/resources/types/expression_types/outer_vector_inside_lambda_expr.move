module 0x1::m {
    fun main(amounts: vector<u8>) {
        let f = |i| {
            let amount = amounts[i];
            amount;
            //^ u8
        };
        
    }
}        