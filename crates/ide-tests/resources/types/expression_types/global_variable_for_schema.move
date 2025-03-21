module 0x1::m {
    spec module {
        global supply: num;
    }
    spec schema MySchema {
        ensures supply == 1;
                  //^ num   
    }
}        