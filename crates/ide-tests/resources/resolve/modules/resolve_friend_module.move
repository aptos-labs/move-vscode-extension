module 0x1::myfriend {}
             //X
module 0x1::main {
    friend 0x1::myfriend;
               //^ 
}    