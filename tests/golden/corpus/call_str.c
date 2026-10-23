// L1 structure recovery (1 blocks)
void call_str_l1(void) {
    rsp = (rsp - 0xADDR);
    str_probe();
    eax = (eax + 1);
    rsp = (rsp + 0xADDR);
    return;
}
