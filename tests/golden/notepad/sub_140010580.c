// L1 structure recovery (12 blocks)
void sub_140010580_l1(void) {
    edi = ecx;
    xmm0 = 0;
    r14d = 0;
    edx = 0;
    var_190 = r14w;
    var_38 = xmm0;
    sub_1400029dc(&var_192, 0, 0xADDR);
    ebx = (r14 + 1);
    GetLocalTime(&var_38);
    var_30 = ebx;
    sub_140010798(0xADDR);
    if (eax != 0) {
        GetLocaleInfoW(0xADDR, 0xADDR, &var_30, (r14 + 2));
        eax = var_30;
        if (eax != ebx) {
            if (eax > 0xADDR) {
                ebx = 0xADDR;
            } else {
                if (((0xADDR >> eax) & 1) != 0) {
                    ebx = 0xADDR;
                }
            }
        } else {
            GetUserDefaultUILanguage();
            sub_140010798((uint32)(ax));
        }
    }
    rax = &var_f0;
    var_28 = 0xADDR;
    var_20 = rax;
    GetDateFormatW(0xADDR, ebx, &var_38, 0, &var_f0, 0xADDR);
    r9d = 0;
    var_28 = 0xADDR;
    rax = &var_50;
    var_20 = rax;
    GetTimeFormatW(0xADDR, (r9 + 2), &var_38, 0, &var_50, 0xADDR);
    ebx = 0xADDR;
    if (edi != 0) {
        sub_1400082b0(&var_190, ebx, "\r");
    }
    sub_1400082b0(&var_190, rbx, &var_50);
    sub_1400082b0(&var_190, rbx, " ");
    sub_1400082b0(&var_190, rbx, &var_f0);
    if (edi != 0) {
        sub_1400082b0(&var_190, rbx, "\r");
    }
    SendMessageW(qword_ADDR, 0xADDR, 1, &var_190);
    sub_1400239c8(edi);
    r11 = &var_440;
    return;
}
