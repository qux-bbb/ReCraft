// L1 structure recovery (1 blocks)
void atexit_l1(void) {
    _onexit(rcx);
    rax = -(rax);
    eax = (((rax != 0) ? 1 : 0) - 1);
    return;
}
