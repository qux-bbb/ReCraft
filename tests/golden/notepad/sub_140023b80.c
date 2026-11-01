// L1 structure recovery (3 blocks)
void sub_140023b80_l1(void) {
    var_40 = 0;
    rbx = (int64)(edx);
    rdi = rcx;
    SHGetKnownFolderPath(off_14002e2d0, 0, 0, &var_40);
    sub_1400082b0(rdi, rbx, var_40);
    sub_1400082b0(rdi, rbx, "\\Microsoft\\WindowsApps\\Microsoft.WindowsNotepad_8wekyb3d8bbwe\\notepad.exe");
    rcx = var_40;
    if (rcx != 0) {
        CoTaskMemFree(var_40);
    }
    return;
}
