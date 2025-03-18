module aptos_std::original {
    friend aptos_std::m;
    public(friend) fun call() {}
                     //X
}
module aptos_std::m {
    use aptos_std::original;
    fun main() {
        original::call();
                 //^
    }
}    