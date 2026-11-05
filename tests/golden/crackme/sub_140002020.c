// L1 structure recovery (8 blocks)
void sub_140002020_l1(void) {
    rdi = *(u64*)(rcx);
    rbx = rcx;
    if (*(u32*)(rdi) == 0xADDR) {
        loc_ADDR:
        sub_14000217c(rcx);
        if (*(u32*)((rax + 0xADDR)) > 0 /* signed */) {
            sub_14000217c(rcx);
            *(u32*)((rax + 0xADDR)) -= 1;
        }
        return;
    }
    if (*(u32*)(rdi) == 0xADDR) goto loc_ADDR;
    if (*(u32*)(rdi) == 0xADDR) {
        sub_14000217c(rcx);
        *(u64*)(0xADDR) = rdi;
        rbx = *(u64*)((rbx + 8));
        sub_14000217c(rcx);
        *(u64*)(0xADDR) = rbx;
        sub_1400087a0();
        eax = 0;
        return;
    }
    eax = 0;
    return;
}
