// L1 structure recovery (6 blocks)
void sub_140007da0_l1(void) {
    rax = qword_ADDR;
    ebx = 0;
    edi = ecx;
    if (rax != 0) {
        ecx = edi;
        sub_140028010(edi);
        ebx = eax;
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "RtlNtStatusToDosErrorNoTeb");
        qword_ADDR = rax;
        if (rax != 0) {
            ecx = edi;
            sub_140028010(edi);
            ebx = eax;
            eax = ebx;
            return;
        }
    }
    eax = ebx;
    return;
}
