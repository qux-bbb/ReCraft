// L1 structure recovery (1 blocks)
void cpp_entry_l1(void) {
    ebx = edi;
    sub_10b0();
    puts(rax);
    edi = ebx;
    sub_10a0();
    edi = ebx;
    ebp = ((rbx + rax) + 3);
    sub_10c0();
    eax += ebp;
    return;
}
