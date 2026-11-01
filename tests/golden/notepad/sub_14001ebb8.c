// L1 structure recovery (3 blocks)
void sub_14001ebb8_l1(void) {
    eax = 0;
    rbx = r8;
    rdi = rdx;
    rsi = rcx;
    if (r8 != 0) {
        wcsnlen(rbx, (int64)(r9d));
    }
    eax = ((rax * 2) + 2);
    var_28 = eax;
    var_20 = rbx;
    RegSetValueExW(rsi, rdi, 0, 1, rbx, ((rax * 2) + 2));
    return;
}
