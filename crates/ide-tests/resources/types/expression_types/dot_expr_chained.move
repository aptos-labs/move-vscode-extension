module 0x1::m {
    struct Pool { field: u8 }
    fun main(pool: &mut Pool) {
        pool.
        //^ &mut 0x1::m::Pool
        pool.field;
    }
}        