module 0x1::m {
    struct Field { id: u8 }
    fun main() {
        let Field { id } = &mut Field { id: 1 };
        id;
       //^ &mut u8 
    }
}                