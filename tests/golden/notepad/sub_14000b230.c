// L1 structure recovery (22 blocks)
void sub_14000b230_l1(void) {
    *(u64*)(r8) = 0;
    if (*(u32*)(rdx) != 0) {
        if (*(u32*)(rdx) != 0xADDR) {
            rcx += 8;
            if (*(u32*)(rdx) != 0xADDR) {
                if (*(u32*)(rdx) != 3) {
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
                        eax = 0xADDR;
                        return;
                    }
                    rax = r8;
                    *(u64*)(rax) = rcx;
                    rcx = *(u64*)(r8);
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 8));
                    sub_140028010();
                }
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
                rax = r8;
                *(u64*)(rax) = rcx;
                rcx = *(u64*)(r8);
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 8));
                sub_140028010();
                eax = ebx;
                return;
            }
        } else {
            eax = dword_ADDR;
            if (*(u32*)((rdx + 4)) == eax) {
                eax = dword_ADDR;
                if (*(u32*)((rdx + 8)) == eax) {
                    eax = dword_ADDR;
                    if (*(u32*)((rdx + 0xADDR)) == eax) {
                        rax = r8;
                        *(u64*)(rax) = rcx;
                        rcx = *(u64*)(r8);
                        rax = *(u64*)(rcx);
                        rax = *(u64*)((rax + 8));
                        sub_140028010();
                        eax = ebx;
                        return;
                    }
                }
            }
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
