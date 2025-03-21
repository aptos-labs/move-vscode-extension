module 0x1::m {
    enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }
    struct Box has drop {
       x: u64
    }
    enum Outer {
        None,
        One{ i: Inner },
        Two { i: Inner, b: Box }
    }
    public fun is_inner1(self: &Inner): bool {
                //X
        match (self) {
            Inner1{x: _} => true,
            _ => false
        }
    }
    public fun main(o: Outer) {
        match (o) {
            None => false
            One { i } if i.is_inner1() => true,
                           //^
        }
    }
}        