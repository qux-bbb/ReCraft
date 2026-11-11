// L1 structure recovery (3 blocks)
void getenv_probe_l1(void) {
    getenv("RECRAFT_CORPUS");
    rdi = rax;
    eax = 0;
    if (rdi != 0) {
        strlen(rdi);
    }
    return;
}
