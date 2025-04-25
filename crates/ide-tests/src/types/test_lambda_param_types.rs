use crate::types::check_expr_type;

#[test]
fn test_for_each_ref_parameter() {
    // language=Move
    check_expr_type(
        r#"
module std::option {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    public inline fun for_each_ref<Element>(self: &Option<Element>, f: |&Element|) {
    }
}
module std::asset {
    use std::option::Option;
    struct FunctionInfo has copy, drop, store {
        module_address: address,
    }
    public fun main(function: Option<FunctionInfo>) {
        function.for_each_ref(|function| {
            function;
            //^ &std::asset::FunctionInfo
        })
    }
}
    "#,
    );
}
