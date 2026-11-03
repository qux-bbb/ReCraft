// L1 structure recovery (7 blocks)
void sub_14000e848_l1(void) {
    ebp = edx;
    esi = ecx;
    if (dword_ADDR != 0) {
        sub_14001f420(rcx);
        xmm0 = 0;
        rect = xmm0;
        GetClientRect(qword_ADDR, &rect);
        edi = rect.bottom;
        edi -= rect.top;
    }
    rcx = qword_ADDR;
    ebx = 0;
    if (rcx != 0) {
        xmm0 = 0;
        rect2 = xmm0;
        GetClientRect(qword_ADDR, &rect2);
        ebx = rect2.bottom;
        ebx -= rect2.top;
    }
    ebp -= ebx;
    ebp -= edi;
    edi = 1;
    var_28 = 1;
    var_20 = ebp;
    MoveWindow(qword_ADDR, 0, ebx, esi, (ebp - edi), 1);
    rcx = qword_ADDR;
    if (rcx != 0) {
        var_28 = edi;
        var_20 = ebx;
        MoveWindow(qword_ADDR, 0, 0, esi, ebx, edi);
    }
    return;
}
