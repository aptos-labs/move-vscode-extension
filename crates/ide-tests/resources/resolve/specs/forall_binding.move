module 0x1::M {
    spec module {
        invariant forall ind in 0..len(bytes)
                       //X
            : ind != 0;
            //^
    }
}    