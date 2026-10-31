// L1 structure recovery (3 blocks)
void sub_140002534_l1(void) {
    ebx = ecx;
    sub_140002354(2, "FlsGetValue", off_1400193a0, "FlsGetValue");
    ecx = ebx;
    if (rax == 0) {
        TlsGetValue();
        return;
    }
    (*qword_ADDR)();
    return;
}
