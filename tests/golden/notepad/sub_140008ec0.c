// L1 structure recovery (7 blocks)
void sub_140008ec0_l1(void) {
    if (((rcx >> 0xADDR) & 1) == 0) {
        if (byte_ADDR != 0) {
            r8 = rcx;
            rcx = off_140034288;
            sub_140008c2c(rcx, off_140034260, r8);
        }
    } else if (byte_ADDR != 0) {
        r8 = 0xADDR;
        r8 &= rcx;
        rcx = off_1400342d8;
        sub_140008c2c(rcx, off_140034260, r8);
        return;
    }
    return;
}
