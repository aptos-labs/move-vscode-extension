module 0x1::string {
    public inline fun foreach<Element>(v: vector<Element>, f: |Element|) {}
                      //X
}
module 0x1::main {
    use 0x1::string::foreach;
    fun main() {
        foreach(vector[1, 2, 3], |e| print(e))
        //^
    }
}