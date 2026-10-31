// L1 structure recovery (5 blocks)
void internal/abi.UseInterfaceSwitchCache_l1(void) {
    if ((rax == 0) || (rax == 2)) {
        eax = 1;
        return;
    }
    if ((rax - 4) > 5) {
        eax = 0;
        return;
    }
}
