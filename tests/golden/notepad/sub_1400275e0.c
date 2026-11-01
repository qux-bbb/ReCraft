// L1 structure recovery (7 blocks)
void sub_1400275e0_l1(void) {
    if (byte_ADDR != 0) {
        rcx = qword_ADDR;
        byte_ADDR = 0;
        if (rcx != 0) {
            sub_140007ab4(rcx, rdx);
        }
        return;
    }
    if (qword_ADDR == 0) {
        rcx = off_140034240;
        return sub_140003dc4();
    }
    sub_140028010();
    if (al != 0) {
        rcx = qword_ADDR;
        byte_ADDR = 0;
        if (rcx != 0) {
            sub_140007ab4(rcx, rdx);
        }
        return;
    }
}
