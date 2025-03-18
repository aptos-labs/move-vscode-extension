module 0x1::M {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
      //X  
    }
    
    public fun is_none<Element>(t: &mut Option<Element>): bool {
        &t.vec;
          //^
    }
}    