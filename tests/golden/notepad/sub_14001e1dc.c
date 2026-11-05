// L1 structure recovery (1 blocks)
void sub_14001e1dc_l1(void) {
    var_34 = 0;
    rdi = rdx;
    var_40 = 0;
    rbx = rcx;
    var_30 = 0xADDR;
    LoadCursorW(0, 0xADDR);
    var_58 = rax;
    LoadIconW(rbx, 2);
    var_28 = 0;
    r9d = 0xADDR;
    var_50 = rax;
    var_20 = 0xADDR;
    LoadImageW(rbx, (r9 - 0xADDR), (r9 - 0xADDR), 0xADDR, 0xADDR, 0);
    var_68 = 1;
    var_78 = rax;
    var_70 = "Notepad";
    var_38 = 0xADDR;
    var_48 = rdi;
    var_60 = 6;
    RegisterClassExW(&var_30);
    eax = (uint32)(ax);
    return;
}
