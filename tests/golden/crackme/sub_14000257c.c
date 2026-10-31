// L1 structure recovery (4 blocks)
void sub_14000257c_l1(void) {
    rbx = rdx;
    edi = ecx;
    sub_140002354(3, "FlsSetValue", off_1400193b8, "FlsSetValue");
    rdx = rbx;
    ecx = edi;
    if (rax == 0) {
        TlsSetValue();
    } else {
        (*qword_ADDR)();
    }
    return;
}
