// L1 structure recovery (8 blocks)
void sub_140024e6c_l1(void) {
    rax = "Security-SPP-GenuineLocalStatus";
    ecx = 0xADDR;
    r9d = 0;
    while (*(u16*)(rax) != r9w) {
        rax += 2;
        rcx -= 1;
        if (rcx == 0) break;
    }
    rax = rcx;
    rax = -(rax);
    eax = ((rax != 0) ? -1 : 0);
    eax = (~(eax) & 0xADDR);
    if (r8 == 0) {
        return;
    }
    if (rcx == 0) {
        *(u64*)(r8) = r9;
    }
    rdx -= rcx;
    *(u64*)(r8) = rdx;
    return;
}
