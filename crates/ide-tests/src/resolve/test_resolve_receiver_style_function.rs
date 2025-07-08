// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::resolve::check_resolve;

// language=Move
#[test]
fn test_friend_function_method() {
    check_resolve(
        r#"
module 0x1::m {
    friend 0x1::main;
    struct S { x: u64 }
    public(friend) fun receiver(self: &S): u64 { self.x }
                       //X
}
module 0x1::main {
    use 0x1::m::S;
    fun main(s: S) {
        s.receiver();
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_receiver_style_with_generic_argument() {
    check_resolve(
        r#"
module 0x1::main {
    struct S<T> { field: T }
    fun receiver<T>(self: S<T>): T {
       //X
        self.field
    }
    fun main(s: S<u8>) {
        s.receiver()
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_friend_function_method_unresolved() {
    check_resolve(
        r#"
module 0x1::m {
    struct S { x: u64 }
    public(friend) fun receiver(self: &S): u64 { self.x }
}
module 0x1::main {
    use 0x1::m::S;
    fun main(s: S) {
        s.receiver();
          //^ unresolved
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_method_cannot_be_resolved_if_self_is_not_the_first_parameter() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(y: u64, self: &S): u64 {
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver(&s)
          //^ unresolved
    }
}                
"#,
    )
}

// language=Move
#[test]
fn test_cannot_be_resolved_if_self_requires_mutable_reference() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: &mut S, y: u64): u64 {
        self.x + y
    }
    fun test_call_styles(s: &S): u64 {
        s.receiver(1)
          //^ unresolved
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_receiver_function_with_inline_mut_reference() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    inline fun receiver_mut_ref(self: &mut S, y: u64): u64 {
              //X
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver_mut_ref(1)
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_receiver_function() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
       //X
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver(1)
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_unresolved_private_receiver_style_from_another_module() {
    check_resolve(
        r#"
module 0x1::m {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
        self.x + y
    }
}
module 0x1::main {
    use 0x1::m::S;
    
    fun test_call_styles(s: S): u64 {
        s.receiver(1)
          //^ unresolved
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_public_receiver_style_from_another_module() {
    check_resolve(
        r#"
module 0x1::m {
    struct S { x: u64 }
    public fun receiver(self: S, y: u64): u64 {
                 //X
        self.x + y
    }
}
module 0x1::main {
    use 0x1::m::S;
    
    fun test_call_styles(s: S): u64 {
        s.receiver(1)
          //^
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_enum_with_receiver_function_from_a_call_expr() {
    check_resolve(
        r#"
module 0x1::m {
    enum Ordering has copy, drop {
        Less,
        Equal,
        Greater,
    }
    native public fun compare<T>(first: &T, second: &T): Ordering;
    public fun is_eq(self: &Ordering): bool {
               //X
        self is Ordering::Equal
    }
    fun main() {
        compare(&1, &1).is_eq();
                       //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_cannot_be_resolved_if_self_has_another_type() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    struct T { x: u64 }
    fun receiver(self: T, y: u64): u64 {
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver(1)
          //^ unresolved
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_method_with_enum_inner_item() {
    check_resolve(
        r#"
module 0x1::m {
    enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }
    struct Box has drop {
       x: u64
    }
    enum Outer {
        None,
        One{ i: Inner },
        Two { i: Inner, b: Box }
    }
    public fun is_inner1(self: &Inner): bool {
                //X
        match (self) {
            Inner1{x: _} => true,
            _ => false
        }
    }
    public fun main(o: Outer) {
        match (o) {
            None => false
            One { i } if i.is_inner1() => true,
                           //^
        }
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_receiver_resolvable_if_self_requires_reference_but_mut_reference_exists() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: &S, y: u64): u64 {
        //X
        self.x + y
    }
    fun test_call_styles(s: &mut S): u64 {
        s.receiver(1)
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_receiver_function_with_reference() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver_ref(self: &S, y: u64): u64 {
       //X
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver_ref(1)
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_public_package_method() {
    check_resolve(
        r#"
module 0x1::m {
    struct S { x: u64 }
    public(package) fun receiver(self: &S): u64 { self.x }
                          //X
}
module 0x1::main {
    use 0x1::m::S;
    fun main(s: S) {
        s.receiver();
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_self_parameter() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
                 //X
        self.x + y
        //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_resolve_receiver_function_with_mut_reference() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver_mut_ref(self: &mut S, y: u64): u64 {
       //X
        self.x + y
    }
    fun test_call_styles(s: S): u64 {
        s.receiver_mut_ref(1)
          //^
    }
}        
"#,
    )
}

// language=Move
#[test]
fn test_cannot_be_resolved_if_self_requires_no_reference() {
    check_resolve(
        r#"
module 0x1::main {
    struct S { x: u64 }
    fun receiver(self: S, y: u64): u64 {
        self.x + y
    }
    fun test_call_styles(s: &mut S): u64 {
        s.receiver(1)
          //^ unresolved
    }
}        
"#,
    )
}
