// L1 structure recovery (8 blocks)
void sub_140001474_l1(void) {
    if (ecx == 0) {
        byte_ADDR = 1;
    }
    sub_140001a34();
    sub_140001ef8();
    if (al != 0) {
        sub_140008754();
        if (al != 0) {
            al = 1;
        } else {
            ecx = 0;
            sub_140001f20(0);
            al = 0;
        }
    } else {
        al = 0;
        return;
    }
    return;
}
