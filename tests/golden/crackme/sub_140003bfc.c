// L1 structure recovery (3 blocks)
void sub_140003bfc_l1(void) {
    rdi = rcx;
    *(u64*)(rcx) = off_1400194a0;
    ebx = edx;
    rcx += 8;
    __std_exception_destroy(rcx);
    if ((bl & 1) != 0) {
        sub_140015c10(rdi, 0xADDR);
    }
    rax = rdi;
    return;
}
