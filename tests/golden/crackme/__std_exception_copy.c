// L1 structure recovery (7 blocks)
void __std_exception_copy_l1(void) {
    rdi = rdx;
    rbx = rcx;
    if (*(u8*)((rcx + 8)) == 0) {
        *(u64*)(rdx) = *(u64*)(rbx);
        *(u8*)((rdx + 8)) = 0;
    } else {
        rcx = *(u64*)(rcx);
        if (rcx == 0) {
            *(u64*)(rdx) = *(u64*)(rbx);
            *(u8*)((rdx + 8)) = 0;
            return;
        }
        strlen(rcx);
        rbp = (rax + 1);
        malloc(rbp);
        rsi = rax;
        rcx = rax;
        if (rax != 0) {
            strcpy_s(rax, rbp, *(u64*)(rbx));
            ecx = 0;
            *(u64*)(rdi) = rsi;
            *(u8*)((rdi + 8)) = 1;
        }
        free(rcx);
    }
    return;
}
