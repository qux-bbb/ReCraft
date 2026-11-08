// L1 structure recovery (3 blocks)
void tailish_l1(void) {
    if (edi <= 0 /* signed */) {
        eax = edi;
        return;
    }
    eax = ((rdi - 1) & -0xADDR);
    eax = ((rdi + rax) - 2);
    return;
}
