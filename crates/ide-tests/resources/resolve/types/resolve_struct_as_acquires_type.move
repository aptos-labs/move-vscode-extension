module 0x1::m {
    struct MyStruct {}
         //X
    
    fun call() acquires MyStruct {}
                      //^
}