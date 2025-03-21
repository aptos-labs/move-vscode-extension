module unknown_address::m1 {}
module 0x1::m {
    use unknown_address::m1;
                       //^ unresolved
}        