; SPDX-License-Identifier: AGPL-3.0-only
; Copyright (c) 2026 ReCraft contributors
;
; Optional NSIS installer for the green zip built by scripts/package.ps1.
;
;   makensis /DVERSION=0.1.0 /DSTAGE="C:\...\dist\ReCraft-0.1.0" /DOUT="C:\...\dist\recraft-0.1.0-setup.exe" packaging\installer.nsi
;
; The binary-first workflow is the portable zip; this installer is a convenience layer.

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef STAGE
  !error "STAGE must point at the staged ReCraft directory"
!endif
!ifndef OUT
  !define OUT "recraft-${VERSION}-setup.exe"
!endif

Name "ReCraft ${VERSION}"
Icon "${STAGE}\recraft.ico"
UninstallIcon "${STAGE}\recraft.ico"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\ReCraft"
RequestExecutionLevel user
Unicode true

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "ReCraft (required)"
  SetOutPath "$INSTDIR"
  File /r "${STAGE}\*.*"
  CreateShortcut "$SMPROGRAMS\ReCraft.lnk" "$INSTDIR\recraft.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "DisplayName" "ReCraft ${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "Publisher" "ReCraft contributors"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "DisplayIcon" "$INSTDIR\recraft.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "NoRepair" 1
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\ReCraft.lnk"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft"
SectionEnd
