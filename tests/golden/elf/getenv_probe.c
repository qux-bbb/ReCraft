// L1 structure recovery (3 blocks)
void getenv_probe_l1(void) {
    rdi = 0xADDR;
    sub_10c0();
    rdi = rax;
    eax = 0;
    if (rdi != 0) {
        sub_1100();
    }
    return;
}
