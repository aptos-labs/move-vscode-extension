module 0x1::M {
    struct T {
        my_field: u8
      //X  
    }

    fun main() {
        let T { my_field: my_field_1 } = call();
              //^
    }
}