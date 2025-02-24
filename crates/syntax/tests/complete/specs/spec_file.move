spec sender::my_module {
    spec create_address {
        pragma opaque;
    }

    spec fun spec_now_microseconds(): u64 {
        global<CurrentTimeMicroseconds>(@aptos_framework).microseconds
    }

    spec schema AbortsIfNotGenesis {
        aborts_if !is_genesis() with error::INVALID_STATE;
    }

    spec write_to_event_store<T: drop + store>() {
    }
}
