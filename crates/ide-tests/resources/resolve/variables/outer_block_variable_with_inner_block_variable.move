module 0x1::m {
    fun main() {
        let supply = 1;
        spec {
            let supply = 2;
                //X
            supply;
            //^ 
        }
    }
}        