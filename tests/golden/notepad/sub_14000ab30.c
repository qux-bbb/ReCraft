// L1 structure recovery (8 blocks)
void sub_14000ab30_l1(void) {
    rbx = rcx;
    if (edx == 0xADDR) {
        if (byte_ADDR == 0) {
            var_30 = 0;
            var_20 = 0xADDR;
            var_24 = 2;
            var_28 = rbx;
            TrackMouseEvent(&var_20);
        }
        byte_ADDR = 1;
        rcx = rbx;
        InvalidateRect(rcx, 0, 1);
        eax = 0;
        return;
    }
    if (edx == 0xADDR) {
        byte_ADDR = 0;
        InvalidateRect(rcx, 0, 1);
        eax = 0;
        return;
    }
    ord_413();
    return;
}
