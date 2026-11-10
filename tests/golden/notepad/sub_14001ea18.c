// L1 structure recovery (7 blocks)
void sub_14001ea18_l1(void) {
    eax = 0;
    var_60 = 1;
    var_50 &= 0;
    rbx = rdx;
    hKey &= 0;
    var_30 = 0;
    var_68 = 4;
    if (rcx == 0) {
        var_60 = eax;
        RegCreateKeyW(0xADDR, "Software\\Microsoft\\Notepad", &hKey);
        if (eax == 0) {
            rax = &var_30;
            var_28 = 4;
            var_20 = rax;
            RegSetValueExW(hKey, rbx, 0, 4, &var_30, 4);
            RegCloseKey(hKey);
        }
    } else {
        *(u64*)((r11 - 0xADDR)) = &var_68;
        rax = &var_60;
        *(u64*)((r11 - 0xADDR)) = rax;
        RegQueryValueExW(rcx, rdx, 0, &var_50);
        if (eax != 0) {
            eax = var_30;
        }
    }
    eax = var_60;
    return;
}
