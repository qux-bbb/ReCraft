// L1 structure recovery (3 blocks)
void sub_140027780_l1(void) {
    rcx = qword_ADDR;
    qword_ADDR = 0;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)(rax);
        sub_140028010(rcx, 1);
    }
    return;
}
