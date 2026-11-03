// L1 structure recovery (11 blocks)
void sub_140025ed0_l1(void) {
    *(u64*)(r8) = 0;
    if (*(u32*)(rdx) != 0) {
        if (*(u32*)(rdx) != 0xADDR) {
            ebx = 0xADDR;
        } else {
            eax = dword_ADDR;
            if (*(u32*)((rdx + 4)) != eax) {
                eax = 0xADDR;
                return;
            }
            eax = dword_ADDR;
            if (*(u32*)((rdx + 8)) != eax) {
                eax = 0xADDR;
                return;
            }
            eax = dword_ADDR;
            if (*(u32*)((rdx + 0xADDR)) != eax) {
                ebx = 0xADDR;
                eax = 0xADDR;
                return;
            }
            *(u64*)(r8) = rcx;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 8));
            sub_140028010();
        }
    } else {
        eax = dword_ADDR;
        if (*(u32*)((rdx + 4)) == eax) {
            eax = dword_ADDR;
            if (*(u32*)((rdx + 8)) == eax) {
                eax = dword_ADDR;
                if (*(u32*)((rdx + 0xADDR)) == eax) {
                    *(u64*)(r8) = rcx;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 8));
                    sub_140028010();
                    eax = ebx;
                    return;
                }
            }
        }
    }
    eax = ebx;
    return;
}
