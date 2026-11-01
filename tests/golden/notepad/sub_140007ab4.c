// L1 structure recovery (30 blocks)
void sub_140007ab4_l1(void) {
    rdi = rcx;
    if (byte_ADDR != 0) {
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        if (eax == 0) {
            sub_1400039e4(&var_20);
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_20, (rdi + 0xADDR));
            }
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_60, (rdi + 0xADDR));
            }
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_a0, (rdi + 0xADDR));
            }
            sub_140006a1c(&var_20);
            sub_1400040c8(&var_20);
        }
        r11 = &var_e0;
        return;
    }
    rax = qword_ADDR;
    if (rax == 0) {
        rbx = *(u64*)((rdi + 8));
        edx = -1;
        WaitForSingleObjectEx(rbx, (edx | -1), 0);
        if (eax == 0xADDR) {
            ebx = 0;
            *(u32*)(rdi) -= 1;
            eax = *(u32*)(rdi);
            if (eax != 0) {
                if (rbx == 0) {
                    r11 = &var_e0;
                    return;
                }
                ReleaseMutex(rbx);
                if (eax == 0) {
                    sub_1400091e4(var_e8, 0xADDR);
                }
                r11 = &var_e0;
                return;
            }
            edx = 0;
            sub_140009488((rdi + 0xADDR), 0);
            edx = 0;
            sub_140009488((rdi + 0xADDR), 0);
            if (rbx == 0) {
                sub_140003cdc((rdi + 0xADDR));
                sub_140003fa0((rdi + 0xADDR));
                rcx = *(u64*)((rdi + 8));
                if (rcx == 0) {
                    GetProcessHeap();
                    HeapFree(rax, 0, rdi);
                    r11 = &var_e0;
                    return;
                }
                CloseHandle(*(u64*)((rdi + 8)));
                if (eax == 0) {
                    sub_1400091e4(var_e8, 0xADDR);
                }
            }
            GetLastError();
            ebp = eax;
            ReleaseMutex(rbx);
            if (eax == 0) {
                sub_1400091e4(var_e8, 0xADDR);
            }
            SetLastError(ebp);
        }
        if ((eax & 0xADDR) != 0) {
            sub_140004f24(var_e8);
        }
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        if (eax != 0) {
            if (rbx == 0) {
                r11 = &var_e0;
                return;
            }
            ReleaseMutex(rbx);
            if (eax == 0) {
                sub_1400091e4(var_e8, 0xADDR);
            }
            r11 = &var_e0;
            return;
        }
        r11 = &var_e0;
        return;
    }
    sub_140028010();
    if (al != 0) {
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        if (eax == 0) {
            sub_1400039e4(&var_20);
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_20, (rdi + 0xADDR));
            }
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_60, (rdi + 0xADDR));
            }
            if (*(u8*)((rdi + 0xADDR)) != 0) {
                sub_1400087e0(&var_a0, (rdi + 0xADDR));
            }
            sub_140006a1c(&var_20);
            sub_1400040c8(&var_20);
        }
        r11 = &var_e0;
        return;
    }
}
