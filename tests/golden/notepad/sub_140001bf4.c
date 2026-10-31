// L1 structure recovery (4 blocks)
void sub_140001bf4_l1(void) {
    bl = cl;
    if (byte_ADDR == 0) {
        sub_140002088(rcx);
        cl = bl;
        sub_140002088(cl);
    } else if (dl == 0) {
        sub_140002088(rcx);
        cl = bl;
        sub_140002088(cl);
        al = 1;
        return;
    }
    al = 1;
    return;
}
