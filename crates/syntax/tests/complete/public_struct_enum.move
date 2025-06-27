module 0x1::public_struct_enum {
    public struct S {}
    public enum S { One, Two }

    public(friend) struct S {}
    public(package) struct S {}
    package struct S {}
}
