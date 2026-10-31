// L1 structure recovery (6 blocks)
void sub_1400019f8_l1(void) {
    rbx = off_140020ff0;
    rdi = off_140020ff0;
    while (rbx < rdi) {
        if (*(u64*)(rbx) != 0) {
            (*qword_ADDR)();
        }
        rbx += 8;
    }
    return;
}
