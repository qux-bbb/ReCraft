// L1 structure recovery (4 blocks)
void sub_1400025d0_l1(void) {
    esi = r8d;
    ebx = edx;
    rdi = rcx;
    sub_140002354(4, "InitializeCriticalSectionEx", off_1400193d0, "InitializeCriticalSectionEx");
    edx = ebx;
    rcx = rdi;
    if (rax == 0) {
        InitializeCriticalSectionAndSpinCount();
    } else {
        r8d = esi;
        (*qword_ADDR)();
    }
    return;
}
