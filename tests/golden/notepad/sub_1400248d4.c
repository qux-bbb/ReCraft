// L1 structure recovery (11 blocks)
void sub_1400248d4_l1(void) {
    rdi = r8;
    ebx = edx;
    rsi = rcx;
    edx = 0;
    sub_1400029dc(&var_20, 0, 0xADDR);
    if (rdi != 0) {
        al = 8;
        if ((*(u8*)((rdi + 0xADDR)) & 8) == 0) {
            if ((*(u8*)((rdi + 0xADDR)) & 0xADDR) != 0) {
                if ((bl & 0xADDR) == 0) {
                    if ((bl & 0xADDR) == 0) {
                        bl &= al;
                        bl = -(bl);
                        edx = (((bl != 0) ? -0xADDR : 0) + 0xADDR);
                    } else {
                        edx = 0xADDR;
                    }
                } else {
                    edx = 0xADDR;
                }
                LoadStringW(qword_ADDR, rdx, &var_20, 0xADDR);
            }
        } else {
            bl &= al;
            bl = -(bl);
            edx = (((bl != 0) ? -1 : 0) + 0xADDR);
            LoadStringW(qword_ADDR, rdx, &var_20, 0xADDR);
            rdx = &var_20;
            sub_1400249b0(rsi);
            r11 = &var_130;
            return;
        }
    }
    rdx = &var_20;
    sub_1400249b0(rsi);
    r11 = &var_130;
    return;
}
