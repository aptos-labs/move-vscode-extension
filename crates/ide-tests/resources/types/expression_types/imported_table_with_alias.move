module 0x1::table_with_length {
    struct TableWithLength<phantom K: copy + drop, phantom V> has store {}
}
module 0x1::pool {
    use 0x1::table_with_length::TableWithLength as Table;
    struct Pool has store {
        shares: Table<address, u128>,
    }

    fun add_shares(pool: &mut Pool) {
        let shares = pool.shares;
        shares;
        //^ 0x1::table_with_length::TableWithLength<address, u128>
    }
}