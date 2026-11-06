// L1 structure recovery (8 blocks)
void fflush_l1(void) {
    var_48 = rcx;
    if (rcx != 0) {
        edx = *(u32*)((rcx + 0xADDR));
        eax = edx;
        al &= 3;
        if (al != 2) {
            if (((edx >> 0xADDR) & 1) != 0) {
                var_58 = &var_48;
                r9 = &var_50;
                rax = var_48;
                var_50 = rax;
                var_20 = rax;
                sub_140004788(&var_40, &var_20, &var_58);
            } else {
                eax = 0;
            }
        } else if ((dl & 0xADDR) != 0) {
            var_58 = &var_48;
            r9 = &var_50;
            rax = var_48;
            var_50 = rax;
            var_20 = rax;
            sub_140004788(&var_40, &var_20, &var_58);
            return;
        }
    } else {
        sub_1400047c8(rcx);
    }
    return;
}
