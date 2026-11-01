// L1 structure recovery (5 blocks)
void sub_14000cb7c_l1(void) {
    rcx = qword_ADDR;
    if (rcx != 0) {
        GlobalFree(qword_ADDR);
    }
    rcx = qword_ADDR;
    if (rcx != 0) {
        GlobalFree(qword_ADDR);
    }
    xmmword_ADDR = 0;
    return;
}
