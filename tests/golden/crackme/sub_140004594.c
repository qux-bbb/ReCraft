// L1 structure recovery (3 blocks)
void sub_140004594_l1(void) {
    sub_1400049f0();
    sub_1400094d4();
    ebx = 0;
    do {
        rcx = qword_ADDR;
        rcx = *(u64*)((rbx + rcx));
        sub_140009588(rcx);
        rcx = (*(u64*)((rbx + qword_ADDR)) + 0xADDR);
        DeleteCriticalSection((rcx + 0xADDR));
        rbx += 8;
    } while (rbx != 0xADDR);
    sub_140008ee0(qword_ADDR);
    qword_ADDR = 0;
    return;
}
