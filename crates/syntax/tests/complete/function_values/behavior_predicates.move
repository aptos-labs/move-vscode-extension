module 0x1::behavior_predicates {
    spec main {
        aborts_of<f>(1);
        requires_of<f>(1);
        ensures_of<f>(1);
        result_of<f>(1);
        ensures result_of<config::call>();
    }
}
