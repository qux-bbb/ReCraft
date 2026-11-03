// L1 structure recovery (1 blocks)
void sub_140023c00_l1(void) {
    rbx = rcx;
    edx = 0;
    sub_1400029dc(&var_e2, 0, 0xADDR);
    eax = 0xADDR;
    var_e0 = ax;
    sub_140023b80(&var_e2, 0xADDR);
    sub_1400082b0(&var_e0, 0xADDR, "\"");
    sub_1400082b0(&var_e0, 0xADDR, rbx);
    edx = 0;
    sub_1400029dc(&var_74, 0, (rdx + 0xADDR));
    var_70 = 0xADDR;
    ebx = 0;
    var_60 = 0;
    rax = &var_50;
    xmm0 = 0;
    var_48 = rax;
    rax = &var_70;
    var_40 = rax;
    var_38 = 0;
    var_30 = 0;
    var_28 = 0;
    var_20 = ebx;
    var_50 = xmm0;
    CreateProcessW(0, &var_e0, 0, 0, 0, 0, 0, 0, &var_70, &var_50, 0);
    bl = (uint8)((eax != 0));
    sub_140023b38(&var_50);
    al = bl;
    return;
}
