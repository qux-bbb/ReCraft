// L1 structure recovery (6 blocks)
void sub_140001ef8_l1(void) {
    sub_1400022d4();
    if (al != 0) {
        sub_140002264();
        if (al != 0) {
            al = 1;
        } else {
            sub_14000231c();
            al = 0;
        }
    } else {
        al = 0;
        return;
    }
    return;
}
