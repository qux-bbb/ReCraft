// L1 structure recovery (21 blocks)
void sub_140008d40_l1(void) {
    eax = edx;
    edx &= 0xADDR;
    if ((ecx != 0) || (r8d != 0) || (edx != 0)) {
        if (((edx >> 0xADDR) & 1) == 0) {
            if (r8d != 0) {
                r9d = r8d;
                r8d = edx;
                edx = ecx;
                sub_140006d34(off_140034240, ecx, edx, r8d);
                return;
            }
            if (edx == 0xADDR) {
                r9d = r8d;
                r8d = edx;
                edx = ecx;
                sub_140006d34(off_140034240, ecx, edx, r8d);
                return;
            }
            var_20 = ecx;
            var_24 = dx;
            if (eax < 0 /* signed */) {
                var_26 |= 1;
            }
            rax = qword_ADDR;
            if (rax != 0) {
                sub_140028010(&var_20);
                return;
            }
            rax = qword_ADDR;
            if (rax == 0) {
                GetModuleHandleW("ntdll.dll");
                qword_ADDR = rax;
            }
            GetProcAddress(rax, "RtlNotifyFeatureUsage");
            qword_ADDR = rax;
            if (rax == 0) {
                return;
            }
        }
        r9d = r8d;
        r8d = (uint32)(dx);
        edx = ecx;
        rcx = off_140034240;
        return sub_1400067a8();
    }
    if (byte_ADDR != dl) return;
    rax = qword_ADDR;
    if (rax == 0) {
        sub_140004cb4(off_140034240);
        if (al == 0) return;
        rdx = qword_ADDR;
        sub_14000647c((rdx + 0xADDR), qword_ADDR);
        rcx = qword_ADDR;
        return sub_140006f64();
    }
    sub_140028010(rcx);
    if (al != 0) return;
    sub_140004cb4(off_140034240);
    if (al == 0) return;
    rdx = qword_ADDR;
    sub_14000647c((rdx + 0xADDR), qword_ADDR);
    rcx = qword_ADDR;
    return sub_140006f64();
}
