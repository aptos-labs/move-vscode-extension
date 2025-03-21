module 0x1::m {
    fun identity<T>(t: T): T { t }
    fun main() {
        (identity(vector[1])[0]);
      //^ integer   
    }
}        