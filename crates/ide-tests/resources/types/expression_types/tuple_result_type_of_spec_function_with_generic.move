module 0x1::m {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    fun upsert(): Option<u8> { Option { vec: vector[1] } }
    spec upsert {
        result;
        //^ 0x1::m::Option<num>
    }
}