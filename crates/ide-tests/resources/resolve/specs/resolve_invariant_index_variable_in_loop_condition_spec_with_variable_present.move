module 0x1::m {
    fun main() {
        let ind = 0;
        while ({
            spec {
                invariant forall ind in 0..10:
                               //X
                    ind < 10;
                  //^
            };
            true
        }) {
            
        }
    }
}        