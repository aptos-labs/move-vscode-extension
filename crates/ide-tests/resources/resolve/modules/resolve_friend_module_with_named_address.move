module aptos_std::myfriend {}
                  //X
module aptos_std::main {
    friend aptos_std::myfriend;
                       //^ 
}    