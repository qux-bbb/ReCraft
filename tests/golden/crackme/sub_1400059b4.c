// L1 structure recovery (8 blocks)
void sub_1400059b4_l1(void) {
    r8 = (rcx + 0xADDR);
    r9 = rdx;
    rax = *(u64*)((r8 + 0xADDR));
    if (rax != 0) {
        r8 = rax;
        rax = *(u64*)((rcx + 0xADDR));
        rax >>= 1;
    } else {
        eax = 0xADDR;
    }
    rdx = (r8 - 1);
    rdx += rax;
    *(u64*)((rcx + 0xADDR)) = rdx;
    r8 = rdx;
    while (1) {
        eax = *(u32*)((rcx + 0xADDR));
        if (eax > 0 /* signed */) {
            eax -= 1;
            *(u32*)((rcx + 0xADDR)) = eax;
            al = r9b;
            al &= 7;
            r9 >>= 3;
            al += 0xADDR;
            *(u8*)(r8) = al;
            *(u64*)((rcx + 0xADDR)) = (*(u64*)((rcx + 0xADDR)) - 1);
            r8 = *(u64*)((rcx + 0xADDR));
            continue;
        }
        if (r9 == 0) break;
    }
    edx -= *(u32*)((rcx + 0xADDR));
    rax = (r8 + 1);
    *(u32*)((rcx + 0xADDR)) = edx;
    *(u64*)((rcx + 0xADDR)) = rax;
    return;
}
