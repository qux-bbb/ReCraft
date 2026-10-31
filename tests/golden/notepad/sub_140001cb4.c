// L1 structure recovery (1 blocks)
void sub_140001cb4_l1(void) {
    rbx = rcx;
    SetUnhandledExceptionFilter(0);
    UnhandledExceptionFilter(rbx);
    GetCurrentProcess();
    TerminateProcess(rax, 0xADDR);
    return;
}
