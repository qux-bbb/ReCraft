// L1 structure recovery (26 blocks)
void sub_140024500_l1(void) {
    ebp = r8d;
    rsi = rdx;
    r8 = *(u64*)((rdx + 0xADDR));
    ebx = 0xADDR;
    rdi = rcx;
    SetDlgItemTextW(rcx, rdx, r8);
    r9d = (uint32)(*(u16*)((rsi + 0xADDR)));
    var_20 = 0;
    r9 -= 1;
    SendDlgItemMessageW(rdi, ebx, 0xADDR, (r9 - 1), 0);
    SendMessageW(rdi, 0xADDR, 0xADDR, 0);
    if ((*(u8*)((rsi + 0xADDR)) & 0xADDR) == 0) {
        GetDlgItem(rdi, (rbx - 0xADDR));
        rbx = rax;
        ShowWindow(rax, 0);
        EnableWindow(rbx, 0);
    }
    r14d = 0xADDR;
    if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) == 0) {
        if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) != 0) {
            GetDlgItem(rdi, r14d);
            rcx = rax;
            EnableWindow(rcx, 0);
        }
    } else {
        GetDlgItem(rdi, r14d);
        rbx = rax;
        ShowWindow(rax, 0);
        rcx = rbx;
        EnableWindow(rcx, 0);
    }
    r8d = ((*(u32*)((rsi + 0xADDR)) >> 1) & 1);
    CheckDlgButton(rdi, r14d, (r8d & 1));
    r14d = 0xADDR;
    if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) == 0) {
        if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) != 0) {
            GetDlgItem(rdi, r14d);
            rcx = rax;
            EnableWindow(rcx, 0);
        }
    } else {
        GetDlgItem(rdi, r14d);
        rbx = rax;
        ShowWindow(rax, 0);
        rcx = rbx;
        EnableWindow(rcx, 0);
    }
    r8d = ((*(u32*)((rsi + 0xADDR)) >> 2) & 1);
    CheckDlgButton(rdi, r14d, (r8d & 1));
    r14d = 0xADDR;
    if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) != 0) {
        if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) != 0) {
            GetDlgItem(rdi, r14d);
            rcx = rax;
            EnableWindow(rcx, 0);
        }
    } else {
        GetDlgItem(rdi, r14d);
        rbx = rax;
        ShowWindow(rax, 0);
        rcx = rbx;
        EnableWindow(rcx, 0);
    }
    r8d = ((*(u32*)((rsi + 0xADDR)) >> 0xADDR) & 1);
    CheckDlgButton(rdi, r14d, (r8d & 1));
    r14d = 0xADDR;
    if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) == 0) {
        if ((*(u32*)((rsi + 0xADDR)) & 0xADDR) != 0) {
            GetDlgItem(rdi, r14d);
            EnableWindow(rax, 0);
            GetDlgItem(rdi, 0xADDR);
            rcx = rax;
            EnableWindow(rcx, 0);
        }
    } else {
        GetDlgItem(rdi, (r14 + 0xADDR));
        ShowWindow(rax, 0);
        GetDlgItem(rdi, r14d);
        rbx = rax;
        ShowWindow(rax, 0);
        EnableWindow(rbx, 0);
        GetDlgItem(rdi, (r14 + 1));
        rbx = rax;
        ShowWindow(rax, 0);
        rcx = rbx;
        EnableWindow(rcx, 0);
    }
    rcx = rdi;
    if (ebp != 0) {
        ebx = 0xADDR;
        SetDlgItemTextW(rdi, ebx, *(u64*)((rsi + 0xADDR)));
        r9d = (uint32)(*(u16*)((rsi + 0xADDR)));
        var_20 = 0;
        r9 -= 1;
        SendDlgItemMessageW(rdi, ebx, 0xADDR, (r9 - 1), 0);
        SendMessageW(rdi, 0xADDR, 0xADDR, 0);
    } else {
        r9d = (*(u32*)((rsi + 0xADDR)) & 1);
        r9d += r14d;
        CheckRadioButton(rdi, r14d, 0xADDR, (r9d + r14d));
    }
    return;
}
