// L1 structure recovery (18 blocks)
void sub_140003dc4_l1(void) {
    rbx = rcx;
    *(u8*)(rcx) = 0;
    rcx += 0xADDR;
    edx = 0;
    sub_1400093f0(rcx, 0);
    edx = 0;
    sub_1400093f0((rbx + 0xADDR), 0);
    rdi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    r8 = *(u64*)((rbx + 0xADDR));
    if (r8 != 0) {
        rdx = qword_ADDR;
        if (rdx != 0) {
            sub_140008c2c((rdx + 0xADDR), rdx, r8);
        }
    }
    rdi = (rbx + 0xADDR);
    rsi = *(u64*)((rdi + 0xADDR));
    *(u64*)((rdi + 0xADDR)) = 0;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    DeleteCriticalSection(rdi);
    rcx = *(u64*)((rbx + 0xADDR));
    if (rcx != 0) {
        sub_140008bb8(rcx);
    }
    rsi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    DeleteCriticalSection((rbx + 0xADDR));
    rdi = *(u64*)((rbx + 0xADDR));
    esi = 1;
    if (rdi != 0) {
        SetThreadpoolTimer(rdi, 0, 0, 0);
        WaitForThreadpoolTimerCallbacks(rdi, esi);
        CloseThreadpoolTimer(rdi);
    }
    rdi = *(u64*)((rbx + 0xADDR));
    if (rdi != 0) {
        SetThreadpoolTimer(rdi, 0, 0, 0);
        WaitForThreadpoolTimerCallbacks(rdi, esi);
        CloseThreadpoolTimer(rdi);
    }
    rcx = *(u64*)((rbx + 0xADDR));
    if (rcx != 0) {
        sub_140007ab4(rcx, rdx);
    }
    return;
}
