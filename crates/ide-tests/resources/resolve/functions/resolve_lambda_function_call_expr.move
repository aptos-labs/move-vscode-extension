module 0x1::mod {
    public inline fun fold<Accumulator, Element>(elem: Element, func: |Element| Accumulator): Accumulator {
                                                               //X
        func(elem);
        //^
    }
}        