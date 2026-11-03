// L1 structure recovery (11 blocks)
void sub_1400046a4_l1(void) {
    rdi = r9;
    rsi = r8;
    sub_140009898(*(u32*)(rdx));
    rbx = qword_ADDR;
    rax = (int64)(dword_ADDR);
    r14 = (rbx + (rax * 8));
    while (1) {
        var_38 = rbx;
        if (rbx == r14) break;
        rcx = *(u64*)(rbx);
        var_20 = rcx;
        rdx = *(u64*)(rsi);
        if (rcx != 0) {
            ecx = *(u32*)((rcx + 0xADDR));
            eax = (ecx >> 0xADDR);
            if ((al & 1) != 0) {
                eax = ecx;
                al &= 3;
                if (al != 2) {
                    if (((ecx >> 0xADDR) & 1) != 0) {
                        rdx = *(u64*)((rsi + 0xADDR));
                        rcx = *(u64*)((rsi + 8));
                        rax = *(u64*)(rsi);
                        r8 = &var_20;
                        var_40 = r8;
                        var_48 = rax;
                        var_50 = rcx;
                        var_58 = rdx;
                        rax = var_20;
                        var_28 = rax;
                        var_30 = rax;
                        r9 = &var_28;
                        sub_140004608(&var_88, &var_30, &var_40);
                    } else {
                        *(u32*)(rdx) += 1;
                    }
                }
            }
        }
        rbx += 8;
        continue;
    }
    sub_1400098ec(*(u32*)(rdi));
    return;
}
