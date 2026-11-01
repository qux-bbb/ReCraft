// L1 structure recovery (4 blocks)
void sub_140010798_l1(void) {
    r9d = 0xADDR;
    GetLocaleInfoW(rcx, (r9 + 0xADDR), &var_20, 0xADDR);
    ecx = 0;
    if (eax != 0) {
        if ((var_2e & 0xADDR) != 0) {
            ecx = 1;
        }
    }
    eax = ecx;
    return;
}
