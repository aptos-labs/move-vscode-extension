module 0x1::string {
    inline fun foreach<Element>(v: vector<Element>, f: |Element|) {}
              //X
    fun main() {
        foreach(vector[1, 2, 3], |e| print(e))
        //^
    }
}        