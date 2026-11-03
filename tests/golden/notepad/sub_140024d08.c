// L1 structure recovery (3 blocks)
void sub_140024d08_l1(void) {
    rbx = rdx;
    rsi = rcx;
    if (*(u64*)(rcx) != 0) {
        rcx = *(u64*)(rcx);
        LocalFree(*(u64*)(rcx));
    }
    rax = rsi;
    rdi = ((rbx != 0) ? rbx : rdi);
    *(u64*)(rsi) = rdi;
    return;
}
