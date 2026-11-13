// L1 structure recovery (14 blocks)
void __vcrt_getptd_noexit_l1(void) {
    if (dword_ADDR != -1) {
        GetLastError();
        edi = eax;
        sub_140002534(dword_ADDR);
        rdx = -1;
        esi = 0;
        if (rax != -1) {
            if (rax == 0) {
                sub_14000257c(dword_ADDR, rdx);
                if (eax != 0) {
                    edx = 0xADDR;
                    calloc((rdx - 0xADDR), 0xADDR);
                    ecx = dword_ADDR;
                    rbp = rax;
                    rbx = rax;
                    if (rax == 0) {
                        sub_14000257c(rcx, 0);
                    } else {
                        sub_14000257c(rcx, rax);
                        if (eax == 0) {
                            ecx = dword_ADDR;
                        } else {
                            *(u32*)((rbx + 0xADDR)) = 0xADDR;
                            rbx = rsi;
                            rsi = rbp;
                        }
                    }
                    free(rbx);
                }
            } else {
                rsi = rax;
            }
        }
        SetLastError(edi);
        rax = rsi;
    } else {
        eax = 0;
    }
    return;
}
