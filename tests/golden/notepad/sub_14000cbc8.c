// L1 structure recovery (3 blocks)
void sub_14000cbc8_l1(void) {
    eax = (rcx - 1);
    SendMessageW(qword_ADDR, 0xADDR, (int64)(eax), 0);
    if (eax != -1) {
        SendMessageW(qword_ADDR, 0xADDR, eax, eax);
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    }
    return;
}
