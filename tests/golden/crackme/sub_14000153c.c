// L1 structure recovery (16 blocks)
void sub_14000153c_l1(void) {
    r8 = rcx;
    eax = 0xADDR;
    if (word_ADDR != 0xADDR) {
        al = 0;
    } else {
        rcx = (int64)(dword_ADDR);
        rdx = 0xADDR;
        rcx += 0xADDR;
        if (*(u32*)(rcx) != 0xADDR) {
            al = 0;
            return;
        }
        eax = 0xADDR;
        if (*(u16*)((rcx + 0xADDR)) != 0xADDR) {
            al = 0;
            return;
        }
        r8 -= rdx;
        edx = (uint32)(*(u16*)((rcx + 0xADDR)));
        rdx += 0xADDR;
        rdx += rcx;
        eax = (uint32)(*(u16*)((rcx + 6)));
        rcx = (rax + (rax * 4));
        r9 = (rdx + (rcx * 8));
        while (1) {
            var_0 = rdx;
            if (rdx == r9) break;
            ecx = *(u32*)((rdx + 0xADDR));
            if (r8 < rcx) {
                rdx += 0xADDR;
                continue;
            }
            eax = *(u32*)((rdx + 8));
            eax += ecx;
        }
        edx = 0;
        if (rdx != 0) {
            if (*(u32*)((rdx + 0xADDR)) >= 0 /* signed */) {
                al = 1;
            } else {
                al = 0;
            }
        } else {
            al = 0;
        }
    }
    return;
}
