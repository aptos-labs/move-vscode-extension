script {
    fun main() {
        let a = 1;
          //X
        {
            let a = 2;
        };
        a;
      //^  
    }
}