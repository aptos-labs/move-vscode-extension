module 0x1::m {
    fun main() {
        let vect = vector[1u8];
        let ind = 1;
        index_of(vect, ind);
        ind;
        //^ integer
    }
}        