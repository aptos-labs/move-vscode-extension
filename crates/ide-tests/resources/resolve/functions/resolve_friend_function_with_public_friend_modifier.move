address 0x1 {
module Original {
    friend 0x1::M;
    public(friend) fun call() {}
                     //X
}

module M {
    use 0x1::Original;
    fun main() {
        Original::call();
                //^
    }
}    
}