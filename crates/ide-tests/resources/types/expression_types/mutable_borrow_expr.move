module 0x1::M {
    fun main(s: signer) {
        &mut s;
      //^ &mut signer 
    }
}    