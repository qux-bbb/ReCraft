// L1 structure recovery (6 blocks)
void sub_140007d20_l1(void) {
    rax = qword_ADDR;
    ebx = 0;
    if (rax != 0) {
        sub_140028010();
        bl = al;
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "RtlDllShutdownInProgress");
        qword_ADDR = rax;
        if (rax != 0) {
            sub_140028010();
            bl = al;
            al = bl;
            return;
        }
    }
    al = bl;
    return;
}
