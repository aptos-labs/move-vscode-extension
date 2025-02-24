module 0x1::paths {
    fun main() {
        a::<T>;
        a::b::<T>;
        a::b::c;
        a::b::c::<T>;

        0x1::m;
        0x1::m::call;
    }
}
