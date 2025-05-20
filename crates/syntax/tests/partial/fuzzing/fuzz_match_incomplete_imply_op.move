module 0x1::match {
    fun t7_unqualified_variant() {
        match (self) {
            Some { value: Option::None } =
        }
    }

    fun t8_unqualified_variant() {
        match (self) {
            Some { value: Option::None } =,
            Some { value: Option::None } =
