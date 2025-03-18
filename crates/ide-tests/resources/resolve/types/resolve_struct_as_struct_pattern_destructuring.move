module 0x1::m {
    struct MyStruct { val: u8 }
         //X
    
    fun call() {
        let MyStruct { val } = get_struct();
          //^
    }
}