module 0x1::M {
    fun call() {}
    spec call {
        let post count = 1;
        let count2 = count + 1;
                      //^ unresolved
    }
}