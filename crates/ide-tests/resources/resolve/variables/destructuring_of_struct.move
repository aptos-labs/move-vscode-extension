module 0x1::M {
    struct MyStruct {
        val: u8
    }
    
    fun destructure() {
        let MyStruct { val } = get_struct();
                     //X
        val;
      //^  
    }
}