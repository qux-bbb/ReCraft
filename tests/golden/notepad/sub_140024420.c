// L1 structure recovery (17 blocks)
void sub_140024420_l1(void) {
    rbx = rcx;
    esi = (uint32)(r8w);
    rdi = rdx;
    GetModuleHandleA("comdlg32.dll");
    ebp = 0;
    if (rax == 0) {
        if (rbx != 0) {
            wcsrchr(rbx, 0xADDR);
            if (rax != 0) {
                rbx = (rax + 2);
            }
        }
        rax = -1;
        if (rbx == 0) {
            r8 = rbp;
        } else {
            r8 = rax;
            do {
                r8 += 1;
            } while (*(u16*)((rbx + (r8 * 2))) != bp);
        }
        r8 += 1;
        if (r8 <= rsi) {
            if (rbx == 0) {
                *(u16*)(rdi) = bp;
            } else {
                r8 += r8;
                rdx = rbx;
                rcx = rdi;
                sub_1400274ac(rdi, rbx, r8);
            }
            eax = (uint32)(bp);
        } else {
            rcx = (int64)(r8w);
            r8w = ((rcx != r8) ? ax : r8w);
            eax = (uint32)(r8w);
        }
    } else {
        GetFileTitleW(rbx, rdi, (uint32)(si));
    }
    return;
}
