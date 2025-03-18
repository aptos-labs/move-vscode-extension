module 0x1::table {
    struct Table {}
           //X
}        
module 0x1::main {
    struct S<phantom T> has key {}
    fun main() {}
}   
spec 0x1::main {
    spec main {
        use 0x1::table::Table;
        
        assert!(exists<S<Table>>(@0x1), 1);
                         //^
    }
}