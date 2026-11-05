// L1 structure recovery (6 blocks)
void sub_14001ec2c_l1(void) {
    var_88 = r9;
    var_80 = r8b;
    rax = qword_ADDR;
    rdi = rcx;
    ebx = (uint32)(dl);
    r8d = *(u32*)(rax);
    if ((r8b & 4) == 0) {
        rdx = (r11 - 0xADDR);
        sub_140012614(rcx);
        var_88 = *(u64*)(rax);
    }
    r9d = r8d;
    var_38 = 3;
    eax = ebx;
    r9d >>= 0xADDR;
    eax ^= 1;
    r8d >>= 0xADDR;
    rcx = (rdi + 8);
    var_74 = 3;
    var_80 = 3;
    r9d &= 1;
    var_70 = 0;
    eax = ((rax * 4) + 2);
    r8d &= 1;
    var_20 = eax;
    sub_14001ed0c((rdi + 8), rdx, r8);
    if (eax != 0) {
        rax = qword_ADDR;
        if (rax != 0) {
            var_38 = 1;
            rcx = &var_80;
            var_30 = dil;
            var_28 = 0;
            r9d = ebx;
            var_20 = rcx;
            r8d = 0;
            sub_140028010(0xADDR, &var_70);
        }
    }
    return;
}
