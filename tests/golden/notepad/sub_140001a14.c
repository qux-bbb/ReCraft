// L1 structure recovery (7 blocks)
void sub_140001a14_l1(void) {
    sub_1400025cc();
    if (eax == 0) {
        al = 0;
    } else {
        __asm("mov rax, gs:[0xADDR]");
        rcx = *(u64*)((rax + 8));
        while (1) {
            eax = 0;
            __asm("lock cmpxchg [0xADDR], rcx");
            if (qword_ADDR == 0) break;
            if (rcx != rax) continue;
        }
    }
    return;
}
