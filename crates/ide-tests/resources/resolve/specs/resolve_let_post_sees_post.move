module 0x1::M {
    fun call() {}
    spec call {
        let post count = 1;
                //X
        let post count2 = count + 1;
                         //^ 
    }
}