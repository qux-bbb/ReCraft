// L1 structure recovery (4 blocks)
void sub_140006ec0_l1(void) {
    rax = qword_ADDR;
    edx |= 0xADDR;
    if (rax != 0) {
        r9d = 0;
        sub_140028010();
    } else {
        rax = qword_ADDR;
        if (rax != 0) {
            r9d = 0;
            sub_140028010();
            return;
        }
    }
    return;
}
