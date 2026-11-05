// L1 structure recovery (10 blocks)
void sub_140001094_l1(void) {
    r9d = 0;
    r10 = rdx;
    rcx = -1;
    r8d = (r9 + 2);
    rax = var_118;
    var_b8 = 4;
    var_a8 = 4;
    var_98 = 8;
    rdx = *(u64*)(rax);
    if (rdx == 0) {
        rdx = off_14002b210;
        eax = r8d;
    } else {
        rax = rcx;
        do {
            rax += 1;
        } while (*(u16*)((rdx + (rax * 2))) != 0xADDR);
        eax = ((rax * 2) + 2);
    }
    var_88 = eax;
    rax = var_110;
    var_80 = rdx;
    var_8c = 0;
    rdx = *(u64*)(rax);
    if (rdx == 0) {
        rdx = off_14002b210;
    } else {
        do {
            rcx += 1;
        } while (*(u16*)((off_14002b210 + (rcx * 2))) != 0xADDR);
        r8d = ((rcx * 2) + 2);
    }
    var_60 = var_108;
    rax = var_100;
    var_70 = rdx;
    var_78 = r8d;
    r8d = 0;
    var_7c = r9d;
    rcx = *(u64*)(rax);
    rax = &var_30;
    var_50 = rcx;
    var_28 = rax;
    var_20 = 9;
    var_68 = 8;
    var_58 = 0xADDR;
    sub_1400012dc(off_1400341f8, r10, 0);
    return;
}
