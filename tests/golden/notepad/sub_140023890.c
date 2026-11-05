// L1 structure recovery (14 blocks)
void sub_140023890_l1(void) {
    r8d = 0;
    if (ecx == 0) {
        r8 = "EditMenu";
    } else if (ecx == 1) {
        r8 = "ContextMenu";
    }
    if (dword_ADDR > 4) {
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = (qword_ADDR & 0xADDR);
            if (rax == qword_ADDR) {
                rax = (qword_ADDR + 1);
                var_30 = edx;
                qword_ADDR = rax;
                var_38 = rax;
                rax = &var_30;
                var_90 = rax;
                var_98 = 4;
                if (r8 == 0) {
                    r8 = off_14002b210;
                    eax = 2;
                } else {
                    rax = -1;
                    do {
                        rax += 1;
                    } while (*(u16*)((r8 + (rax * 2))) != 0xADDR);
                    eax = ((rax * 2) + 2);
                }
                var_88 = eax;
                rax = &var_38;
                var_80 = r8;
                var_70 = rax;
                var_8c = 0;
                var_60 = off_140036178;
                r8d = 0;
                rax = &var_40;
                var_78 = 8;
                var_28 = rax;
                var_20 = 6;
                var_68 = 0xADDR;
                sub_1400012dc(off_1400341f8, off_14002e90e, 0);
            }
        }
    }
    return;
}
