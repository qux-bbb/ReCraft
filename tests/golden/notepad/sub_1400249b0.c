// L1 structure recovery (3 blocks)
void sub_1400249b0_l1(void) {
    rbx = rdx;
    GetDlgItem();
    rdi = rax;
    if (rax != 0) {
        rax = off_14002b210;
        rbx = ((rbx == 0) ? rax : rbx);
        SetWindowTextW(rdi, rbx);
        r9d = 0;
        NotifyWinEvent(0xADDR, rdi, (r9 - 4), 0);
    }
    return;
}
