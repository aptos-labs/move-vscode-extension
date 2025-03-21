module 0x1::m {
    enum BigOrderedMap<K: store, V: store> has store { BPlusTreeMap }
    public native fun borrow<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): &V;
    fun main() {
        let map = BigOrderedMap<vector<u8>, vector<u8>>::BPlusTreeMap;
        borrow(&map, &vector[1]);
        //^ &vector<u8>
    }
}        