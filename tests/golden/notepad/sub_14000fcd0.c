// L1 structure recovery (4 blocks)
void sub_14000fcd0_l1(void) {
    rbx = rcx;
    sub_1400029dc(&var_20, 0, 0xADDR);
    sub_1400083f8(&var_20, 0xADDR, rbx, r9);
    if (eax >= 0 /* signed */) {
        rcx = qword_ADDR;
        if (rcx != 0) {
            SendMessageW(qword_ADDR, 0xADDR, 0, &var_20);
        }
    }
    return;
}
