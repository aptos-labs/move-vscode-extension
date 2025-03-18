module 0x1::M {
    struct T {
        my_field: u8
      //X  
    }

    fun main() {
        let t = T { my_field: 1 };
                  //^
    }
}