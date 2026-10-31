// L1 structure recovery (11 blocks)
void sub_140001260_l1(void) {
    r11 = rcx;
    rcx = var_80;
    if (rcx != 0) {
        r10 = var_70;
        eax = edx;
        if (edx == 0) {
            *(u32*)(rcx) = 0;
        } else if (eax == 1) {
            if (r8b == 0) {
                eax = 0xADDR;
            } else {
                eax = (uint32)(r8b);
                eax += 1;
            }
            *(u32*)(rcx) = eax;
            *(u64*)((rcx + 0xADDR)) = r9;
            *(u64*)((rcx + 0xADDR)) = r10;
        }
        if (*(u64*)((rcx + 0xADDR)) != 0) {
            rcx = *(u64*)((rcx + 0xADDR));
            var_30 = rcx;
            var_28 = var_78;
            rcx = r11;
            var_20 = r10;
            sub_140028010();
        }
    }
    return;
}
