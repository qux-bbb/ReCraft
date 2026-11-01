// L1 structure recovery (12 blocks)
void sub_140006a8c_l1(void) {
    dl = *(u8*)((rcx + 0xADDR));
    rbx = rcx;
    ecx = (*(u32*)((rcx + 0xADDR)) - 2);
    if (ecx == 0) {
        rcx = *(u64*)((rbx + 0xADDR));
        rdi = (rbx + 0xADDR);
        rdx = *(u64*)((rdi + 0xADDR));
        rax = (rcx + 8);
        rdx = ((rdx == 0) ? rdi : rdx);
        *(u64*)((rbx + 0xADDR)) = rax;
        al = *(u8*)(rcx);
        *(u8*)(rdx) = al;
        *(u32*)((rbx + 0xADDR)) = 1;
    } else {
        ecx -= 1;
        if (ecx == 0) {
            rcx = *(u64*)((rbx + 0xADDR));
            *(u64*)((rbx + 0xADDR)) = (rcx + 8);
            rdx = *(u64*)((rbx + 0xADDR));
            rax = *(u64*)((rbx + 8));
            r9d = (uint32)(*(u16*)(rcx));
            if (rdx != 0) {
                r8 = *(u64*)((rbx + 0xADDR));
                rdi = (rbx + 0xADDR);
                r8 >>= 1;
            } else {
                rdx = (rbx + 0xADDR);
                r8d = 0xADDR;
                rdi = rdx;
            }
            var_20 = rax;
            sub_14000c348((rbx + 0xADDR), rdx, r8, r9, rax);
            if (eax != 0) {
                *(u8*)((rbx + 0xADDR)) = 1;
            }
        } else if (ecx != 9) {
            if (*(u32*)((rbx + 0xADDR)) == 0xADDR) {
                rcx = *(u64*)((rbx + 0xADDR));
                rdi = (rbx + 0xADDR);
                rdx = *(u64*)((rdi + 0xADDR));
                rax = (rcx + 8);
                rdx = ((rdx == 0) ? rdi : rdx);
                *(u64*)((rbx + 0xADDR)) = rax;
                *(u8*)(rdx) = *(u8*)(rcx);
                *(u32*)((rbx + 0xADDR)) = 1;
                rcx = *(u64*)((rdi + 0xADDR));
                al = 1;
                rcx = ((rcx == 0) ? rdi : rcx);
                *(u64*)((rbx + 0xADDR)) = rcx;
                return;
            }
            dl -= 0xADDR;
            al = (uint8)(((dl & 0xADDR) != 0));
            if (al == 0) {
                rcx = *(u64*)((rbx + 0xADDR));
                rdi = (rbx + 0xADDR);
                rdx = *(u64*)((rdi + 0xADDR));
                rax = (rcx + 8);
                rdx = ((rdx == 0) ? rdi : rdx);
                *(u64*)((rbx + 0xADDR)) = rax;
                *(u8*)(rdx) = *(u8*)(rcx);
                *(u32*)((rbx + 0xADDR)) = 1;
                rcx = *(u64*)((rdi + 0xADDR));
                al = 1;
                rcx = ((rcx == 0) ? rdi : rcx);
                *(u64*)((rbx + 0xADDR)) = rcx;
                return;
            }
        }
    }
    rcx = *(u64*)((rdi + 0xADDR));
    al = 1;
    rcx = ((rcx == 0) ? rdi : rcx);
    *(u64*)((rbx + 0xADDR)) = rcx;
    return;
}
