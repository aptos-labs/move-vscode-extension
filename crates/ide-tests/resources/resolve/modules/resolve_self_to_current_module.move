module 0x1::transaction {
            //X
    fun create() {}
    fun main() {
        let a = Self::create();
              //^
    }
}