// L1 structure recovery (10 blocks)
void sub_14002346c_l1(void) {
    if (edx == 0) {
        if (dword_ADDR > 4) {
            rdx = 0xADDR;
            if ((qword_ADDR & 0xADDR) != 0) {
                rax = qword_ADDR;
                rax &= rdx;
                if (rax == qword_ADDR) {
                    rax = qword_ADDR;
                    rdx = off_14002eb2b;
                    rax += 1;
                    qword_ADDR = rax;
                    var_a0 = rax;
                    var_a8 = off_140036178;
                    var_28 = &var_a0;
                    rax = &var_a8;
                    var_20 = rax;
                    sub_140001008(rcx);
                }
            }
        }
    } else if (dword_ADDR > 4) {
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = (qword_ADDR & 0xADDR);
            if (rax == qword_ADDR) {
                var_98 = *(u32*)((rcx + 0xADDR));
                var_a0 = *(u32*)((rcx + 4));
                eax = *(u32*)(rcx);
                var_a8 = rax;
                var_60 = *(u64*)((rcx + 8));
                var_68 = (rcx + 0xADDR);
                rax = (qword_ADDR + 1);
                qword_ADDR = rax;
                var_70 = rax;
                var_78 = off_140036178;
                *(u64*)((r11 - 0xADDR)) = &var_98;
                *(u64*)((r11 - 0xADDR)) = &var_a0;
                *(u64*)((r11 - 0xADDR)) = &var_a8;
                *(u64*)((r11 - 0xADDR)) = &var_60;
                *(u64*)((r11 - 0xADDR)) = &var_68;
                *(u64*)((r11 - 0xADDR)) = &var_70;
                rax = &var_78;
                *(u64*)((r11 - 0xADDR)) = rax;
                sub_140001094(rcx, off_14002eb62);
            }
        }
    }
    return;
}
