module 0x1::consts {
    const ERR: u8 = 1;
    const V: vector<u8> = vector[1, 2];
    const B: vector<u8> = b"123";
    const X: vector<u8> = x"ff11";

    public const PUBLIC_C1: u8 = 2;
    package const PUBLIC_C2: u8 = 2;
    friend const PUBLIC_C3: u8 = 2;
    public(package) const PUBLIC_C4: u8 = 2;
}
