// L1 structure recovery (5 blocks)
void sub_140027590_l1(void) {
    if (byte_ADDR != 0) {
        rcx = off_140034c38;
        return sub_1400066f4();
    }
    if (qword_ADDR == 0) {
        rcx = off_140034c38;
        return sub_140003ba4();
    }
    sub_140028010();
    if (al != 0) {
        rcx = off_140034c38;
        return;
    }
}
