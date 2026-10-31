// L1 structure recovery (1 blocks)
void sub_1400016d0_l1(void) {
    sub_1400016f4();
    *(u64*)(rax) = (*(u64*)(rax) | 4);
    sub_140001704();
    *(u64*)(rax) = (*(u64*)(rax) | 2);
    eax = 0;
    return;
}
