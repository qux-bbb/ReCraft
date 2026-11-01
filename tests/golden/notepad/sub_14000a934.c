// L1 structure recovery (7 blocks)
void sub_14000a934_l1(void) {
    var_20 = 0;
    r9d = 3;
    ebx = edx;
    edi = ecx;
    ImageList_Create(rcx, rdx, (r9 + 0xADDR), 3, 0);
    rsi = rax;
    ImageList_SetBkColor(rax, 0xADDR);
    rax = &hIcon;
    hIcon = 0;
    var_20 = rax;
    ord_381(qword_ADDR, 0xADDR, edi, ebx, &hIcon);
    edx = -1;
    ImageList_ReplaceIcon(rsi, (edx | -1), hIcon);
    rax = &var_78;
    var_78 = 0;
    var_20 = rax;
    ord_381(qword_ADDR, 0xADDR, edi, ebx, &var_78);
    edx = -1;
    ImageList_ReplaceIcon(rsi, (edx | -1), var_78);
    rax = &var_70;
    var_70 = 0;
    var_20 = rax;
    ord_381(qword_ADDR, 0xADDR, edi, ebx, &var_70);
    edx = -1;
    ImageList_ReplaceIcon(rsi, (edx | -1), var_70);
    rcx = var_70;
    if (rcx != 0) {
        DestroyIcon(hIcon);
    }
    rcx = var_78;
    if (rcx != 0) {
        DestroyIcon(*(u64*)((rbp + 0xADDR)));
    }
    rcx = hIcon;
    if (rcx != 0) {
        DestroyIcon(var_n10);
    }
    rax = rsi;
    return;
}
