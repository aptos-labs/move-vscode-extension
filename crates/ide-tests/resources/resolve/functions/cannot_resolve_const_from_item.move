address 0x1 {
module Original {
    const MY_CONST: u8 = 1;
}

module M {
    use 0x1::Original::MY_CONST;
    fun main() {
        MY_CONST;
        //^ unresolved
    }
}    
}