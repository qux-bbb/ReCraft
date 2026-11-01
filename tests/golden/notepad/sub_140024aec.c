// L1 structure recovery (14 blocks)
void sub_140024aec_l1(void) {
    rdi = (rdx + 0xADDR);
    rsi = rdx;
    *(u32*)(rdi) = (*(u32*)(rdi) & 0xADDR);
    r14d = r9d;
    r15d = r8d;
    rbp = rcx;
    IsDlgButtonChecked(rcx, rdx);
    if (eax == 0) {
        rbx = rdi;
    } else {
        *(u32*)(rdi) = (*(u32*)(rdi) | 2);
        rbx = (rsi + 0xADDR);
    }
    IsDlgButtonChecked(rbp, 0xADDR);
    if (eax == 0) {
        rbx = rdi;
    } else {
        *(u32*)(rdi) = (*(u32*)(rdi) | 4);
    }
    IsDlgButtonChecked(rbp, 0xADDR);
    if (eax != 0) {
        eax = (*(u32*)(rdi) | 0xADDR);
        *(u32*)(rbx) = eax;
    }
    r9d = (uint32)(*(u16*)((rsi + 0xADDR)));
    r8 = *(u64*)((rsi + 0xADDR));
    *(u32*)(rbx) = (*(u32*)(rbx) | r15d);
    GetDlgItemTextW(rbp, 0xADDR, *(u64*)((rsi + 0xADDR)), (uint32)(*(u16*)((rsi + 0xADDR))));
    rcx = rbp;
    if (r14d != 0) {
        GetDlgItemTextW(rbp, 0xADDR, *(u64*)((rsi + 0xADDR)), (uint32)(*(u16*)((rsi + 0xADDR))));
        *(u32*)(rbx) = (*(u32*)(rbx) | 1);
    } else {
        IsDlgButtonChecked(rbp, 0xADDR);
        if (eax == 0) {
            *(u32*)(rbx) = (*(u32*)(rbx) | 1);
            return;
        }
    }
    return;
}
