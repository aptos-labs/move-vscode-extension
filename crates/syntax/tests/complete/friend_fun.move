module 0x1::friend_fun {
    friend fun main1() {}
    friend inline fun main2() {}
    friend native fun main3();
    friend entry fun main4();
}
