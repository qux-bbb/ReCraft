// L1 structure recovery (19 blocks)
void sub_140002354_l1(void) {
    edi = ecx;
    r15 = 0xADDR;
    r14 = -1;
    r12 = r9;
    rbp = r8;
    r13 = rdx;
    rax = *(u64*)(((0xADDR + (rdi * 8)) + 0xADDR));
    if (rax == -1) {
        eax = 0;
    } else if (rax == 0) {
        if (r8 == r9) {
            __asm("xchg [r15+rdi*8+0xADDR], r14");
        } else {
            while (1) {
                esi = *(u32*)(rbp);
                rbx = *(u64*)(((r15 + (rsi * 8)) + 0xADDR));
                if (rbx == 0) {
                    r15 = *(u64*)(((r15 + (rsi * 8)) + 0xADDR));
                    LoadLibraryExW(r15, 0, 0xADDR);
                    rbx = rax;
                    if (rax == 0) {
                        GetLastError();
                        if (eax != 0xADDR) {
                            rax = -1;
                            r15 = 0xADDR;
                            __asm("xchg [r15+rsi*8+0xADDR], rax");
                            loc_ADDR:
                            rbp += 4;
                            if (rbp != r12) continue;
                        } else {
                            sub_1400088d0(r15, "api-ms-", (rbx + 7));
                            if (eax != 0) {
                                LoadLibraryExW(r15, 0, 0);
                                rbx = -1;
                            }
                        }
                    }
                } else {
                    if (rbx != r14) break;
                    goto loc_ADDR;
                }
            }
            GetProcAddress(rbx, r13);
            if (rax == 0) {
                __asm("xchg [r15+rdi*8+0xADDR], r14");
                return;
            }
            rcx = 0;
            __asm("xchg [r15+rdi*8+0xADDR], rcx");
        }
    }
    return;
}
