// L1 structure recovery (5 blocks)
void sub_140008cd0_l1(void) {
    r9d = edx;
    edx &= 0xADDR;
    r9d >>= 7;
    r9d &= 1;
    var_30 = 0;
    eax = (rdx - 2);
    edx = ecx;
    r8b = (uint8)((eax <= 1));
    eax = 0;
    var_40 = 0;
    sub_140005e90(&var_30, ecx, (uint8)((eax <= 1)), r9, r8);
    ecx = 0;
    ecx = ((al != 0) ? var_30 : ecx);
    if (var_40 != 0) {
        ecx |= 0xADDR;
    }
    if (var_44 != 0) {
        ecx |= 0xADDR;
    }
    eax = ecx;
    return;
}
