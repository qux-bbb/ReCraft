// L1 structure recovery (21 blocks)
void sub_14001eee0_l1(void) {
    hKey = 0;
    xmm0 = 0;
    rbx = qword_ADDR;
    eax = 0;
    rcx = -1;
    var_28 = 0xADDR;
    var_38 = xmm0;
    var_48 = 0;
    esi = 0;
    var_50 = 0;
    SetThreadDpiAwarenessContext((rcx | -1));
    rdi = 0;
    GetWindowPlacement(rbx, &var_28);
    ebx = eax;
    SetThreadDpiAwarenessContext(rdi);
    if (ebx == 0) {
        if (dword_ADDR != esi) {
            RegCreateKeyW(0xADDR, "S", &hKey);
            if (eax == 0) {
                if (dword_ADDR != eax) {
                    r8d = dword_ADDR;
                    rdx = "lfEscapement";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "lfOrientation";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "lfWeight";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfItalic";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfUnderline";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfStrikeOut";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfCharSet";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfOutPrecision";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfClipPrecision";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfQuality";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfPitchAndFamily";
                    sub_14001eb80(hKey);
                    sub_14001ebb8(hKey, "lfFaceName", off_14003566c, 0xADDR);
                    r8d = dword_ADDR;
                    rdx = "iPointSize";
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    r8d = dword_ADDR;
                    rdx = "fWrap";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "StatusBar";
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    ebx = 0xADDR;
                    sub_14001ebb8(hKey, "szHeader", off_1400356e0, ebx);
                    sub_14001ebb8(hKey, "szTrailer", off_140035730, ebx);
                    r8d = dword_ADDR;
                    rdx = "iMarginTop";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginBottom";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginLeft";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginRight";
                    sub_14001eb80(hKey);
                }
                if (esi != 0) {
                    r8d = var_44;
                    rdx = "iWindowPosX";
                    sub_14001eb80(hKey);
                    r8d = var_48;
                    rdx = "iWindowPosY";
                    sub_14001eb80(hKey);
                    r8d = var_4c;
                    rdx = "iWindowPosDX";
                    r8d -= var_44;
                    sub_14001eb80(hKey);
                    r8d = var_50;
                    rdx = "iWindowPosDY";
                    r8d -= var_48;
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fReverse";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fWrapAround";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fMatchCase";
                    sub_14001eb80(hKey);
                    ebx = 0xADDR;
                    sub_14001ebb8(hKey, "searchString", off_140036540, ebx);
                    sub_14001ebb8(hKey, "replaceString", off_140036440, ebx);
                }
                RegCloseKey(hKey);
            }
        }
    } else {
        rax = qword_ADDR;
        if (rax != var_44) {
            esi = 1;
            RegCreateKeyW(0xADDR, "S", &hKey);
            if (eax == 0) {
                if (dword_ADDR != eax) {
                    r8d = dword_ADDR;
                    rdx = "lfEscapement";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "lfOrientation";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "lfWeight";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfItalic";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfUnderline";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfStrikeOut";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfCharSet";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfOutPrecision";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfClipPrecision";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfQuality";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "lfPitchAndFamily";
                    sub_14001eb80(hKey);
                    sub_14001ebb8(hKey, "lfFaceName", off_14003566c, 0xADDR);
                    r8d = dword_ADDR;
                    rdx = "iPointSize";
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    r8d = dword_ADDR;
                    rdx = "fWrap";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "StatusBar";
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    ebx = 0xADDR;
                    sub_14001ebb8(hKey, "szHeader", off_1400356e0, ebx);
                    sub_14001ebb8(hKey, "szTrailer", off_140035730, ebx);
                    r8d = dword_ADDR;
                    rdx = "iMarginTop";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginBottom";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginLeft";
                    sub_14001eb80(hKey);
                    r8d = dword_ADDR;
                    rdx = "iMarginRight";
                    sub_14001eb80(hKey);
                }
                if (esi != 0) {
                    r8d = var_44;
                    rdx = "iWindowPosX";
                    sub_14001eb80(hKey);
                    r8d = var_48;
                    rdx = "iWindowPosY";
                    sub_14001eb80(hKey);
                    r8d = var_4c;
                    rdx = "iWindowPosDX";
                    r8d -= var_44;
                    sub_14001eb80(hKey);
                    r8d = var_50;
                    rdx = "iWindowPosDY";
                    r8d -= var_48;
                    sub_14001eb80(hKey);
                }
                if (dword_ADDR != 0) {
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fReverse";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fWrapAround";
                    sub_14001eb80(hKey);
                    r8d = (uint32)(byte_ADDR);
                    rdx = "fMatchCase";
                    sub_14001eb80(hKey);
                    ebx = 0xADDR;
                    sub_14001ebb8(hKey, "searchString", off_140036540, ebx);
                    sub_14001ebb8(hKey, "replaceString", off_140036440, ebx);
                }
                RegCloseKey(hKey);
            }
            r11 = &var_60;
            return;
        }
        rax = qword_ADDR;
    }
    r11 = &var_60;
    return;
}
