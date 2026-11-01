// L1 structure recovery (4 blocks)
void sub_14000d990_l1(void) {
    if (dword_ADDR == 0) {
        SendMessageW(qword_ADDR, 0xADDR, 0, 0);
        al = (uint8)((rax != 0));
    } else {
        r9d = 0;
        SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
        if (rax != 0) {
            SendMessageW(qword_ADDR, 0xADDR, 0, 0);
            al = (uint8)((rax != 0));
            return;
        }
    }
    return;
}
