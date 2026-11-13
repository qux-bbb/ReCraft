// L1 structure recovery (6 blocks)
void _RTC_Initialize_l1(void) {
    rbx = off_140020fe0;
    rdi = off_140020fe0;
    while (rbx < rdi) {
        if (*(u64*)(rbx) != 0) {
            (*qword_ADDR)();
        }
        rbx += 8;
    }
    return;
}
