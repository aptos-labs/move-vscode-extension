module 0x1::m {
    enum Ordering has copy, drop {
        Less,
        Equal,
        Greater,
    }
    native public fun compare<T>(first: &T, second: &T): Ordering;
    public fun is_eq(self: &Ordering): bool {
               //X
        self is Ordering::Equal
    }
    fun main() {
        compare(&1, &1).is_eq();
                       //^
    }
}        