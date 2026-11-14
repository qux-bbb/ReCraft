// L1 structure recovery (1 blocks)
void sub_14001e1dc_l1(void) {
    var_4c = 0;
    rdi = rdx;
    var_40 = 0;
    rbx = rcx;
    var_50 = 0xADDR;
    LoadCursorW(0, 0xADDR);
    var_28 = rax;
    LoadIconW(rbx, 2);
    var_58 = 0;
    r9d = 0xADDR;
    var_30 = rax;
    var_60 = 0xADDR;
    LoadImageW(rbx, (r9 - 0xADDR), (r9 - 0xADDR), 0xADDR, 0xADDR, 0);
    var_18 = 1;
    var_8 = rax;
    var_10 = "Notepad";
    var_48 = 0xADDR;
    var_38 = rdi;
    var_20 = 6;
    RegisterClassExW(&var_50);
    eax = (uint32)(ax);
    return;
}
