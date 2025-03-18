module 0x1::M {
    struct MyStruct {
        val: u8
    }
    
    fun destructure() {
        let MyStruct { val: myval } = get_struct();
                          //X
        myval;
      //^  
    }
}