// L1 structure recovery (7 blocks)
void sub_14001f57c_l1(void) {
    eax = (uint32)(*(u16*)(rcx));
    rsi = rdx;
    rdi = rcx;
    if (ax == 0) {
        eax = 0;
    } else {
        rdi -= rdx;
        while (1) {
            CharUpperW((uint32)(ax));
            ecx = (uint32)(*(u16*)(rsi));
            rbx = rax;
            rsi += 2;
            CharUpperW((uint32)(*(u16*)(rsi)));
            if (bx != ax) break;
            eax = (uint32)(*(u16*)((rdi + rsi)));
            if (ax != 0) continue;
        }
        eax = 1;
    }
    return;
}
