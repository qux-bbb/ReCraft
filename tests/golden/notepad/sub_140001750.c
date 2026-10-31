// L1 structure recovery (7 blocks)
void sub_140001750_l1(void) {
    sub_140002964(2);
    sub_140002068();
    ecx = eax;
    sub_140002970(eax);
    sub_14000205c();
    ebx = eax;
    sub_140002898();
    *(u32*)(rax) = ebx;
    sub_140001a94(1);
    if (al == 0) {
        sub_140002100(7);
    }
    sub_140002384();
    sub_140001c64(sub_1400023d0);
    sub_140002050();
    ecx = eax;
    sub_1400028ec(eax);
    if (eax != 0) {
        sub_140002100(7);
        return;
    }
    sub_140002074();
    sub_1400020a0();
    sub_1400020a0();
    sub_14000205c();
    ecx = eax;
    sub_1400028e0(eax);
    sub_140002088();
    if (al != 0) {
        sub_140002928();
    }
    sub_14000205c();
    sub_1400022a8();
    if (eax != 0) {
        sub_140002100(7);
        return;
    }
    return;
}
