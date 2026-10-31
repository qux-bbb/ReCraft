// L1 structure recovery (6 blocks)
void sub_1400023d0_l1(void) {
    rbx = off_14002f2f8;
    rdi = off_14002f2f8;
    while (rbx < rdi) {
        if (*(u64*)(rbx) != 0) {
            sub_140028010();
        }
        rbx += 8;
    }
    return;
}
