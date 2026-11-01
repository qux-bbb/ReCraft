// L1 structure recovery (3 blocks)
void sub_140005460_l1(void) {
    if (byte_ADDR == 0) {
        edx = 0;
        sub_140005830(sub_140006d20, 0, off_140035430, 0xADDR);
        byte_ADDR = 1;
    }
    rax = off_140035430;
    return;
}
