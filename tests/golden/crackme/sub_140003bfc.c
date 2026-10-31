// L1 structure recovery (3 blocks)
void sub_140003bfc_l1(void) {
    rax = off_1400194a0;
    rdi = rcx;
    *(u64*)(rcx) = rax;
    ebx = edx;
    rcx += 8;
    sub_140004390(rcx);
    if ((bl & 1) != 0) {
        sub_140015c10(rdi, 0xADDR);
    }
    rax = rdi;
    return;
}
