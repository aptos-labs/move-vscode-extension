module 0x1::simple_map {
    struct SimpleMap<Value> has copy, drop, store {
        data: vector<Value>,
    }

    /// Create an empty vector.
    native public fun vector_empty<Element>(): vector<Element>;
    
    public fun create<FunValue: store>(): SimpleMap<FunValue> {
        SimpleMap {
            data: vector_empty(),
        }
    }
    
    fun main() {
        let map = create<u64>();
        let map_data = &map.data;
        map_data;
        //^ &vector<u64>
    }
}        