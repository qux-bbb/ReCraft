// L1 structure recovery (3 blocks)
void sub_14000d9f0_l1(void) {
    ebx = 0;
    hObject = 0;
    GetCurrentProcess();
    OpenProcessToken(rax, (rbx + 8), &hObject);
    if (eax != 0) {
        r9d = (rbx + 4);
        rax = &var_48;
        var_48 = r9d;
        var_20 = rax;
        var_40 = ebx;
        GetTokenInformation(hObject, (rbx + 0xADDR), &var_40, (rbx + 4), &var_48);
        ebx = ((eax != 0) ? var_40 : ebx);
        CloseHandle(hObject);
    }
    eax = ebx;
    return;
}
