module 0x1::match_fq {
    fun t1(self: Color): bool {
        match (self) {
            0x1::Color::Red => true,
            0x1::Color::Blue { blue }  => false,

            std::Color::Red => true,
            std::Color::Blue { blue }  => false,
        }
    }
}
