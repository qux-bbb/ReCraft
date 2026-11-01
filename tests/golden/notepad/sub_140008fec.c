// L1 structure recovery (7 blocks)
void sub_140008fec_l1(void) {
    rax = qword_ADDR;
    rbx = rcx;
    if (rax != 0) {
        sub_140028010(rcx);
    }
    rax = qword_ADDR;
    if (rax != 0) {
        rcx = rbx;
        sub_140028010(rbx);
    }
    edx = 0;
    sub_1400029dc(&var_20, 0, 0xADDR);
    rcx = *(u64*)((rbx + 0xADDR));
    r8d = 1;
    var_38 = r8d;
    var_20 = 0xADDR;
    var_24 = r8d;
    var_40 = 7;
    if (rcx != 0) {
        rax = (int64)(*(u32*)((rbx + 8)));
        r8d = 0;
        var_48 = rax;
        eax = *(u32*)((rbx + 0xADDR));
        var_30 = rcx;
        var_50 = rax;
        var_38 = 3;
        sub_14000909c(&var_20);
    }
    sub_14000909c(&var_20);
}
