module 0x1::M {
    fun main() {
        let a = {
            let b = 1;
              //X
            b + 1
          //^  
        };
    }
}        