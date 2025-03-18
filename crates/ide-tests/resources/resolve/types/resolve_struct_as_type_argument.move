module 0x1::m {
    struct MyStruct {}
             //X
    
    fun call() {
        let a = move_from<MyStruct>();
                        //^
    }
}