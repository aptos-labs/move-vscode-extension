module 0x1::m {
    fun main() {
        let ind = 1;
        || {
            || {
                ind;
               //^ integer     
            };
        }
    }
}        