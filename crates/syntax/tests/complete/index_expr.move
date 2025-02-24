module 0x1::index_expr {
    struct X<M> has copy, drop, store {
        value: M
    }

    struct Y<T> has key, drop {
        field: T
    }

    fun test_vector() {
        let v = vector[x, x];
        v[0].value == 2;
    }

    fun test_vector_borrow_mut() {
        let v = vector[y1, y2];
        (&mut v[0]).field.value = false;
        (&mut v[1]).field.value = true;
        (&v[0]).field.value == false;
        (&v[1]).field.value == true;
    }

    fun test_resource_3(){
        use 0x42::test;
        (&test::Y<X<bool>>[@0x1]).field.value == true;
    }

    fun test_resource_4() {
        let addr = @0x1;
        let y = &mut 0x42::test::Y<X<bool>> [addr];
        y.field.value = false;
        spec {
            assert Y<X<bool>>[addr].field.value == false;
        };
        (&Y<X<bool>>[addr]).field.value == false;
    }

    fun test_resource_5() {
        let addr = @0x1;
        let y = &mut 0x42::test::Y<X<bool>> [addr];
        y.field.value = false;
        let y_resource = Y<X<bool>>[addr];
        y_resource.field.value == false;
    }
}
