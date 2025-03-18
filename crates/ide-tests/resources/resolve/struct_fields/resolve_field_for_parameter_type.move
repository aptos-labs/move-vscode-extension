module 0x1::M {
    struct Cap<phantom Feature> has key { root: address }
                                         //X
    fun m<Feature>(cap: Cap<Feature>) {
        cap.root;
          //^          
    }
}    