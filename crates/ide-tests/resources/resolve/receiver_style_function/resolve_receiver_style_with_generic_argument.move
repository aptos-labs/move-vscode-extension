module 0x1::main {
    struct S<T> { field: T }
    fun receiver<T>(self: S<T>): T {
       //X
        self.field
    }
    fun main(s: S<u8>) {
        s.receiver()
          //^
    }
}        