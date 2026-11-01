// L1 structure recovery (3 blocks)
void sub_1400243c8_l1(void) {
    rdi = qword_ADDR;
    rbx = r8;
    FindResourceExW(rdi, 5);
    if (rax == 0) {
        FindResourceExW(rdi, (rax + 5), rbx, 0);
    }
    return;
}
