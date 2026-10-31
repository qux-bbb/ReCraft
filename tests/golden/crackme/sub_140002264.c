// L1 structure recovery (6 blocks)
void sub_140002264_l1(void) {
    sub_1400024a4(sub_14000215c);
    dword_ADDR = eax;
    if (eax == -1) {
        al = 0;
    } else {
        sub_14000257c(eax, off_140023bb0);
        if (eax == 0) {
            sub_1400022ac();
        } else {
            dword_ADDR = 0xADDR;
            al = 1;
        }
    }
    return;
}
