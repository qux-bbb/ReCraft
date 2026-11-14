// L1 structure recovery (6 blocks)
void _RTC_Initialize_l1(void) {
    rbx = off_14002f2e8;
    rdi = off_14002f2e8;
    while (rbx < rdi) {
        if (*(u64*)(rbx) != 0) {
            sub_140028010();
        }
        rbx += 8;
    }
    return;
}
