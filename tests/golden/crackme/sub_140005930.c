// L1 structure recovery (10 blocks)
void sub_140005930_l1(void) {
    r9 = (rcx + 0xADDR);
    r11b = r8b;
    rax = *(u64*)((r9 + 0xADDR));
    r10d = edx;
    if (rax != 0) {
        r9 = rax;
        rax = (*(u64*)((rcx + 0xADDR)) >> 1);
    } else {
        eax = 0xADDR;
    }
    rdx = (r9 - 1);
    rdx += rax;
    *(u64*)((rcx + 0xADDR)) = rdx;
    r8 = rdx;
    while (1) {
        eax = *(u32*)((rcx + 0xADDR));
        if (eax > 0 /* signed */) {
            eax -= 1;
            *(u32*)((rcx + 0xADDR)) = eax;
            al = (r10b & 0xADDR);
            r10d >>= 4;
            al += 0xADDR;
            r9b = al;
            if (al > 0xADDR) {
                al = (((r11b ^ 1) << 5) + 7);
                al += r9b;
            }
            *(u8*)(r8) = al;
            *(u64*)((rcx + 0xADDR)) -= 1;
            r8 = *(u64*)((rcx + 0xADDR));
            continue;
        }
        if (r10d == 0) break;
    }
    edx -= *(u32*)((rcx + 0xADDR));
    rax = (r8 + 1);
    *(u32*)((rcx + 0xADDR)) = edx;
    *(u64*)((rcx + 0xADDR)) = rax;
    return;
}
