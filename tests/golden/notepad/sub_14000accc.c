// L1 structure recovery (24 blocks)
void sub_14000accc_l1(void) {
    rect = 0;
    GetClientRect(qword_ADDR, &rect);
    esi = rect.right;
    esi -= rect.left;
    GetDpiForWindow(qword_ADDR);
    edi = eax;
    eax = 0;
    r12b = al;
    ebx = (rax + 0xADDR);
    MulDiv((rax + 0xADDR), edi, ebx);
    MulDiv((rbx - 0xADDR), edi, ebx);
    var_30 = 0;
    r15d = 0;
    MulDiv(0xADDR, edi, ebx);
    r8d = ebx;
    edx = edi;
    if (esi > eax /* signed */) {
        ecx = 0xADDR;
    } else {
        r12b = 1;
        ecx = (rbx - 0xADDR);
    }
    MulDiv(rcx, rdx, r8);
    r14d = eax;
    if (edi == dword_ADDR) {
        eax = 0;
        var_28 = 0;
        var_20 = r14d;
        MoveWindow(qword_ADDR, 0, 0, esi, r14d, 0);
    } else {
        dword_ADDR = edi;
        sub_14000a934(r13d, r15d);
        rbp = qword_ADDR;
        if (rbp != 0) {
            GetLastError();
            ebx = eax;
            ImageList_Destroy(rbp);
            SetLastError(ebx);
        }
        edx = 0;
        qword_ADDR = 0;
        sub_1400029dc(&var_54, 0, 0xADDR);
        r9d = 0;
        var_20 = edi;
        var_50 = 0xADDR;
        SystemParametersInfoForDpi((r9 + 0xADDR), 0xADDR, &var_50, 0, edi);
        CreateFontIndirectW(&var_1e8);
        rbp = qword_ADDR;
        r15 = rax;
        if (rbp != 0) {
            GetLastError();
            ebx = eax;
            DeleteObject(rbp);
            SetLastError(ebx);
        }
        rcx = qword_ADDR;
        qword_ADDR = r15;
        SendMessageW(qword_ADDR, 0xADDR, r15, 0);
        r15d = 0;
        var_28 = 0;
        var_20 = r14d;
        MoveWindow(qword_ADDR, 0, 0, esi, r14d, 0);
        sub_14000b160();
        GetDC(qword_ADDR);
        r14 = rax;
        CreateCompatibleDC(rax);
        rbx = rax;
        SelectObject(rax, qword_ADDR);
        rdx = qword_ADDR;
        rbp = -1;
        r8 = rbp;
        rect = r15;
        do {
            r8 += 1;
        } while (*(u16*)((rdx + (r8 * 2))) != r15w);
        GetTextExtentPoint32W(rbx, rdx, r8, &rect);
        rdx = qword_ADDR;
        do {
            rbp += 1;
        } while (*(u16*)((rdx + (rbp * 2))) != r15w);
        GetTextExtentPoint32W(rbx, rdx, rbp, off_140035500);
        ebp = 0xADDR;
        MulDiv((rbp - 0xADDR), edi, ebp);
        edx = rect.left;
        dword_ADDR = eax;
        dword_ADDR = r15d;
        eax = (rdx + (rax * 2));
        dword_ADDR = eax;
        MulDiv((rbp - 0xADDR), edi, ebp);
        dword_ADDR = eax;
        if (rbx != 0) {
            DeleteDC(rbx);
        }
        if (r14 != 0) {
            ReleaseDC(0, r14);
        }
        r15d = var_30;
        ebx = ebp;
    }
    r8d = ebx;
    edx = edi;
    if (r12b == 0) {
        MulDiv(2, edi, ebx);
        r14d = dword_ADDR;
    } else {
        MulDiv(0xADDR, edi, ebx);
        r14d = dword_ADDR;
    }
    ebp = eax;
    MulDiv(0xADDR, edi, ebx);
    ecx = dword_ADDR;
    r8d = 0xADDR;
    ebx = (rcx + (rax * 2));
    MulDiv((r8 - 0xADDR), edi, 0xADDR);
    var_28 = 1;
    var_20 = eax;
    MoveWindow(qword_ADDR, r14d, ebp, ebx, eax, 1);
    r8d = 0xADDR;
    edx = edi;
    ecx = (r8 - 0xADDR);
    if (r12b == 0) {
        ecx = (r8 - 0xADDR);
    }
    MulDiv(rcx, edi, 0xADDR);
    r8d = 0xADDR;
    ebx = eax;
    MulDiv((r8 - 0xADDR), edi, 0xADDR);
    esi -= eax;
    var_28 = 1;
    esi -= 0;
    var_20 = r15d;
    MoveWindow(qword_ADDR, esi, ebx, 0, r15d, 1);
    r11 = &var_260;
    return;
}
