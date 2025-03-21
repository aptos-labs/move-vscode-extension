module 0x1::m {
    spec module {
        fun eq_push_back<Element>(v1: vector<Element>, v2: vector<Element>, e: Element): bool {
            let res = 
            (len(v1) == len(v2) + 1 &&
                v1[len(v1)-1] == e &&
                v1[0..len(v1)-1] == v2[0..len(v2)]);
            res
            //^ bool
        }
    }
}        