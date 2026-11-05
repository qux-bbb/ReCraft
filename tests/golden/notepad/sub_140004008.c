// L1 structure recovery (4 blocks)
void sub_140004008_l1(void) {
    rdi = *(u64*)((rcx + 0xADDR));
    r14d = 0;
    eax = (uint32)(*(u16*)((rcx + 0xADDR)));
    rbp = rcx;
    rsi = ((rax + (rax * 4)) << 4);
    rsi += rdi;
    if (rdi != rsi) {
        rdi += 0xADDR;
        do {
            rbx = *(u64*)(rdi);
            GetProcessHeap();
            HeapFree(rax, 0, rbx);
            *(u64*)(rdi) = 0;
            *(u64*)((rdi + 8)) = 0;
            rdi += 0xADDR;
            rax = (rdi - 0xADDR);
        } while (rax != rsi);
    }
    rbx = *(u64*)((rbp + 0xADDR));
    GetProcessHeap();
    HeapFree(rax, 0, rbx);
    *(u32*)((rbp + 0xADDR)) = 0;
    *(u64*)((rbp + 0xADDR)) = r14;
    return;
}
