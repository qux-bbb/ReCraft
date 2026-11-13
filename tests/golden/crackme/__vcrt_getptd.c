// L1 structure recovery (3 blocks)
void __vcrt_getptd_l1(void) {
    __vcrt_getptd_noexit(rcx);
    if (rax == 0) {
        abort();
    }
    return;
}
