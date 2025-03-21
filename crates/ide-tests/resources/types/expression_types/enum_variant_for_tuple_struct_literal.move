module 0x1::m {
    enum S<T> { One(T) }
    fun main() {
        let s = S::One(true);
        s; 
      //^ 0x1::m::S<bool>   
    }
}        