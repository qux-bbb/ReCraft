// L1 structure recovery (1 blocks)
void sub_14000f5b8_l1(void) {
    rbx = rcx;
    SetThreadDpiAwarenessContext(-0xADDR);
    rdi = rax;
    PageSetupDlgW(rbx);
    ebx = eax;
    SetThreadDpiAwarenessContext(rdi);
    eax = ebx;
    return;
}
