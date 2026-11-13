// L1 structure recovery (8 blocks)
void sub_140002020_l1(void) {
    rdi = *(u64*)(rcx);
    rbx = rcx;
    if (*(u32*)(rdi) == 0xADDR) {
        __vcrt_getptd(rcx);
        if (*(u32*)((rax + 0xADDR)) > 0 /* signed */) {
            __vcrt_getptd(rcx);
            *(u32*)((rax + 0xADDR)) -= 1;
        }
        eax = 0;
        return;
    }
    if (*(u32*)(rdi) == 0xADDR) {
        __vcrt_getptd(rcx);
        if (*(u32*)((rax + 0xADDR)) > 0 /* signed */) {
            __vcrt_getptd(rcx);
            *(u32*)((rax + 0xADDR)) -= 1;
        }
        eax = 0;
        return;
    }
    if (*(u32*)(rdi) == 0xADDR) {
        __vcrt_getptd(rcx);
        *(u64*)((rax + 0xADDR)) = rdi;
        rbx = *(u64*)((rbx + 8));
        __vcrt_getptd(rcx);
        *(u64*)((rax + 0xADDR)) = rbx;
        sub_1400087a0();
        eax = 0;
        return;
    }
    eax = 0;
    return;
}
