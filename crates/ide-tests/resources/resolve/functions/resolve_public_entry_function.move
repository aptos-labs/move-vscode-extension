address 0x1 {
module Original {
    public entry fun call() {}
                     //X
}
}

script {
    use 0x1::Original;
    fun main() {
        Original::call();
                //^
    }
}    