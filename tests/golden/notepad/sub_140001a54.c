// L1 structure recovery (8 blocks)
void sub_140001a54_l1(void) {
    if (ecx == 0) {
        byte_ADDR = 1;
    }
    sub_140002418();
    sub_140002088(rcx);
    if (al != 0) {
        sub_140002088(rcx);
        if (al != 0) {
            al = 1;
        } else {
            ecx = 0;
            sub_140002088(0);
            al = 0;
        }
    } else {
        al = 0;
        return;
    }
    return;
}
