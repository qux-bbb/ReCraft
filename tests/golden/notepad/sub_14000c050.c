// L1 structure recovery (17 blocks)
void sub_14000c050_l1(void) {
    rax = *(u64*)((rcx + 8));
    ebx = 0;
    rsi = *(u64*)(r9);
    rdi = *(u64*)(r8);
    rdx = *(u64*)(rdx);
    r10 = *(u64*)(rax);
    rax = *(u64*)((rcx + 0xADDR));
    rcx = *(u64*)(rax);
    if (r10 == 0) {
        GetModuleFileNameW(rcx, rdx, edi);
        r8d = eax;
        edx = eax;
        if (eax == 0) {
            cl = (uint8)((r8d == 0));
            al = bl;
        } else {
            cl = bl;
            if (rdx < rdi) {
                al = 1;
            }
        }
        if (cl == 0) {
            if (al == 0) {
                rdx = (rdi + rdi);
            } else {
                rdx += 1;
            }
            *(u64*)(rsi) = rdx;
        } else {
            sub_140007cd0(var_28, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource/wil/win32_helpers.h");
            ebx = eax;
        }
    } else {
        r8 = rdx;
        rdx = rcx;
        K32GetModuleFileNameExW(r10, rcx, rdx, edi);
        edx = eax;
        if (eax != 0) {
            rax = (rdi - 1);
            cl = bl;
            if (rdx < rax) {
                rdx += 1;
                *(u64*)(rsi) = rdx;
                eax = ebx;
                return;
            }
            al = bl;
            eax = ebx;
            return;
        }
        cl = 1;
        al = bl;
        eax = ebx;
        return;
    }
    eax = ebx;
    return;
}
