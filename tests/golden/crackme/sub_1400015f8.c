// L1 structure recovery (4 blocks)
void sub_1400015f8_l1(void) {
    bl = cl;
    if (byte_ADDR == 0) {
        sub_140008768();
        cl = bl;
        sub_140001f20(cl);
    } else if (dl == 0) {
        sub_140008768();
        cl = bl;
        sub_140001f20(cl);
        al = 1;
        return;
    }
    al = 1;
    return;
}
