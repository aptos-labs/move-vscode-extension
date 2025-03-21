module 0x1::m {
    struct Field { id: u8 }
    fun main() {
        let Field { id } = &Field { id: 1 };
        id;
       //^ &u8 
    }
}                