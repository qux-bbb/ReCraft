// L1 structure recovery (3 blocks)
void sub_1400024a4_l1(void) {
    rbx = rcx;
    ecx = 0;
    sub_140002354(0, "FlsAlloc", off_140019378, "FlsAlloc");
    if (rax == 0) {
        TlsAlloc();
        return;
    }
    rcx = rbx;
    (*qword_ADDR)();
    return;
}
