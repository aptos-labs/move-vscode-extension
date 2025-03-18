address 0x1 {
    module Original {
        public fun call() {}
                 //X
    }
}
address 0x2 {
    module M {
        use 0x1::Original::call;
        
        fun main() {
            call();
          //^  
        }
    }
}