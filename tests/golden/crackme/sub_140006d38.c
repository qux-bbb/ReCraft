// L1 structure recovery (15 blocks)
void sub_140006d38_l1(void) {
    rsi = var_60;
    rdi = r9;
    rbp = (int64)(r8d);
    r14 = rdx;
    rbp += rdx;
    r15 = rcx;
    rbx = *(u64*)((rsi + 0xADDR));
    if (rdx != rbp) {
        while (1) {
            rax = *(u64*)(r15);
            edx = (int32)(*(u8*)(r14));
            ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
            if ((cl & 1) == 0) {
                ecx = edx;
                r8 = rsi;
                sub_14000acfc(edx, *(u64*)(r15));
                if (eax != -1) {
                    loc_ADDR:
                    *(u32*)(rdi) = (*(u32*)(rdi) + 1);
                    r14 += 1;
                    if (r14 != rbp) continue;
                } else {
                    if (*(u8*)((rsi + 0xADDR)) == 0) break;
                    if (*(u32*)((rsi + 0xADDR)) != 0xADDR) break;
                    rax = *(u64*)(r15);
                    ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
                    if ((cl & 1) == 0) goto loc_ADDR;
                }
            } else {
                rax = *(u64*)(r15);
            }
        }
            rax = *(u64*)(r15);
            if (*(u64*)((rax + 8)) == 0) goto loc_ADDR;
            loc_ADDR:
            r8 = rsi;
            sub_14000acfc(0xADDR, *(u64*)(r15));
            *(u32*)(rdi) = (*(u32*)(rdi) | -1);
        *(u32*)(rdi) = (*(u32*)(rdi) | -1);
    }
    *(u64*)((rsi + 0xADDR)) = rbx;
    return;
}
