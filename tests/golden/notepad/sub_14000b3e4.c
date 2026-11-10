// L1 structure recovery (7 blocks)
void sub_14000b3e4_l1(void) {
    var_50 = 0;
    rax = &var_50;
    var_58 = 4;
    var_28 = rax;
    var_20 = 0;
    RegGetValueW(0xADDR, "Software\\Microsoft\\Notepad", "ShowStoreBanner", 0xADDR, 0, &var_50, &var_58);
    if (eax != 0) {
        var_28 = 0;
        var_20 = 0;
        _o__beginthreadex(0, 0, sub_14000b490, 0, 0, 0);
        if (rax != 0) {
            CloseHandle(rax);
            eax = 0;
        } else {
            eax = 0xADDR;
        }
    } else if (var_50 == eax) {
        eax = 0;
        return;
    }
    return;
}
