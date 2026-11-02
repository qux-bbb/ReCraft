// L1 structure recovery (23 blocks)
void sub_140007924_l1(void) {
    rdi = rcx;
    if (byte_ADDR != 0) {
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        return;
    }
    rax = qword_ADDR;
    if (rax == 0) {
        rbx = *(u64*)((rdi + 8));
        edx = -1;
        WaitForSingleObjectEx(rbx, (edx | -1), 0);
        if (eax == 0xADDR) {
            ebx = 0;
        } else if ((eax & 0xADDR) != 0) {
            sub_140004f24(var_28);
        }
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        if (eax != 0) {
            if (rbx == 0) {
                return;
            }
            ReleaseMutex(rbx);
            if (eax == 0) {
                sub_1400091e4(var_28, 0xADDR);
            }
            return;
        }
        edx = 0;
        sub_140009488((rdi + 0xADDR), 0);
        edx = 0;
        sub_140009488((rdi + 0xADDR), 0);
        if (rbx == 0) {
            sub_140003b20((rdi + 0xADDR));
            sub_140003fa0((rdi + 0xADDR));
            rcx = *(u64*)((rdi + 8));
            if (rcx == 0) {
                GetProcessHeap();
                HeapFree(rax, 0, rdi);
                return;
            }
            CloseHandle(*(u64*)((rdi + 8)));
            if (eax == 0) {
                sub_1400091e4(var_28, 0xADDR);
            }
        }
        GetLastError();
        ebp = eax;
        ReleaseMutex(rbx);
        if (eax == 0) {
            sub_1400091e4(var_28, 0xADDR);
        }
        SetLastError(ebp);
    }
    sub_140028010();
    if (al != 0) {
        *(u32*)(rdi) -= 1;
        eax = *(u32*)(rdi);
        return;
    }
}
