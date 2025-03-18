module 0x1::myfriend {
    public fun call() {}
}
module 0x1::main {
    use 0x1::myfriend;
    
    friend myfriend::call;
                     //^ unresolved 
}    