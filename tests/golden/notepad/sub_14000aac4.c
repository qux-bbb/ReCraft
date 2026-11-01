// L1 structure recovery (1 blocks)
void sub_14000aac4_l1(void) {
    var_40 = 0;
    rax = &var_40;
    var_28 = 4;
    RegSetKeyValueW(0xADDR, "S", "ShowStoreBanner", 4, &var_40, 4);
    DestroyWindow(qword_ADDR);
    qword_ADDR = 0;
    sub_14000b160();
    return;
}
