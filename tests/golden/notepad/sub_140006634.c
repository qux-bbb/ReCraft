// L1 structure recovery (8 blocks)
void sub_140006634_l1(void) {
    rbx = rdx;
    edi = ecx;
    GetProcessHeap();
    rsi = rax;
    HeapAlloc(rax, edi, rbx);
    rbx = rax;
    rax = qword_ADDR;
    if (rax != 0) {
        rdx = rbx;
        rcx = rsi;
        sub_140028010(rsi, rbx);
    } else if (byte_ADDR == al) {
        GetModuleHandleW("ntdll.dll");
        if (rax == 0) {
            rax = qword_ADDR;
        } else {
            GetProcAddress(rax, "RtlDisownModuleHeapAllocation");
            qword_ADDR = rax;
        }
        byte_ADDR = 1;
        if (rax != 0) {
            rdx = rbx;
            rcx = rsi;
            sub_140028010(rsi, rbx);
            rax = rbx;
            return;
        }
    }
    rax = rbx;
    return;
}
