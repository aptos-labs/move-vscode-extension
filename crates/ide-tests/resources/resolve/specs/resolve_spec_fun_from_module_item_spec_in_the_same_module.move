module 0x1::m {
    fun call<T>() {}
}
spec 0x1::m {
    spec module {
        fun deserializable<T>(bytes: vector<u8>): bool;
            //X
    }
    spec call<T>() {
        deserializable<T>();
        //^
    }
}        