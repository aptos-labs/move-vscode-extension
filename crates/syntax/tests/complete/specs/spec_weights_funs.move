module 0x1::spec_weights_funs {
    spec fun pow(base: num, exp: num): num [weight = 20] {
        if (exp == 0) 1 else base * pow(base, exp - 1)
    }
    spec module {
        fun pow(base: num, exp: num): num [weight = 20] {
            if (exp == 0) 1 else base * pow(base, exp - 1)
        }
    }
}
