module 0x1::M {
    fun call() {}
    spec call {
        let count2 = count + 1;
                    //^ unresolved
        let count = 1;
    }
}