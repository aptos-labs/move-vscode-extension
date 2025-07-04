// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::resolve::check_resolve;

#[test]
fn test_resolve_variable() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let a = 1;
          //X
        a;
      //^
    }
}
    "#,
    );
}

#[test]
fn test_variable_declaration_is_unresolved() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let a = 1;
          //^ unresolved
    }
}
    "#,
    );
}

#[test]
fn test_resolve_variable_from_tuple_pattern() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let (a, b);
           //X
        a;
      //^
    }
}
    "#,
    );
}

#[test]
fn test_resolve_variable_with_shadowing() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let a = 1;
        let a = 1;
          //X
        a;
      //^
    }
}
    "#,
    );
}

#[test]
fn test_cannot_resolve_variable_if_declared_after_reference() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        a;
      //^ unresolved
        let a = 1;
    }
}
    "#,
    );
}

#[test]
fn test_resolve_function_parameter() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main(a: u8) {
           //X
        a;
      //^
    }
}
    "#,
    );
}

#[test]
fn test_resolve_const() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    const ERR: u8 = 1;
         //X
    fun main() {
        ERR;
      //^
    }
}
    "#,
    );
}

#[test]
fn test_resolve_function() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun call() {
      //X
    }
    fun main() {
        call();
      //^
    }
}
    "#,
    );
}

#[test]
fn test_resolve_function_in_tail_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun call() {
      //X
    }
    fun main() {
        call()
      //^
    }
}
    "#,
    );
}
