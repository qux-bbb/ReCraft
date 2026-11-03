// L1 structure recovery (7 blocks)
void sub_1400065a0_l1(void) {
    eax = *(u32*)(rcx);
    rbx = rcx;
    if (eax != 0) {
        rdi = (rcx + 8);
        AcquireSRWLockExclusive(rdi);
        rax = *(u64*)((rbx + 0xADDR));
        r8 = *(u64*)((rbx + 0xADDR));
        while (rax != r8) {
            ecx = *(u32*)(rax);
            ecx = -(ecx);
            rcx = *(u64*)((rax + 8));
            edx = (((ecx != 0) ? 0xADDR : 0) + 0xADDR);
            *(u32*)(rcx) &= edx;
            rax += 0xADDR;
            continue;
        }
        rax = *(u64*)((rbx + 0xADDR));
        *(u64*)((rbx + 0xADDR)) = rax;
        edx = (*(u32*)((rbx + 0xADDR)) + 1);
        eax = ((edx != 0) ? edx : eax);
        *(u32*)((rbx + 0xADDR)) = eax;
        if (rdi != 0) {
            ReleaseSRWLockExclusive(rdi);
        }
    }
    return;
}
