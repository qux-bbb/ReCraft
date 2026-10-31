// L1 structure recovery (1 blocks)
void sub_140001170_l1(void) {
    var_58 = rdx;
    rdi = rcx;
    rsi = &var_58;
    sub_140004580(1);
    rbx = rax;
    sub_140001000();
    r9d = 0;
    var_20 = rsi;
    sub_140006e08(*(u64*)(rax), rbx, rdi, 0, &var_58);
    return;
}
