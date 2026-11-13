// L1 structure recovery (7 blocks)
void sub_140004828_l1(void) {
    rdi = rcx;
    if (*(u64*)(rcx) != 0) {
        rax = *(u64*)(rdi);
        return;
    }
    GetLastError();
    var_30 = eax;
    if (*(u8*)((rdi + 0xADDR)) != 0) {
        rdx = *(u64*)((rdi + 8));
    } else {
        *(u64*)((rdi + 8)) = 0;
        edx = 0;
        *(u8*)((rdi + 0xADDR)) = 1;
    }
    sub_14000abec(&var_30);
    ecx = var_30;
    rbx = rax;
    *(u64*)(rdi) = rax;
    SetLastError(var_30);
    if (rbx == 0) {
        abort();
    }
}
