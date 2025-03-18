address 0x1 {
module Original {
    entry fun call() {}
}
}

script {
    use 0x1::Original;
    fun main() {
        Original::call();
                //^ unresolved
    }
}    