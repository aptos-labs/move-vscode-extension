module 0x1::A {
    friend 0x1::B; 
    public ( friend) fun call_a() {}
                        //X
}        
module 0x1::B {
    use 0x1::A;
    
    fun main() {
        A::call_a();
              //^
    }
}