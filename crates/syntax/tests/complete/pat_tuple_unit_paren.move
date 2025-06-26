module 0x1::pat_tuple_unit_paren {
    fun main() {
        let () = ();
        let () = 1;
        let (a) = 1;
        let (a,) = 1;
        let ((a),) = 1;
    }
}
