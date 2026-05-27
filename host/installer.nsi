Unicode true
ManifestDPIAware true

!define APP_NAME "BetterCPT"
!define APP_VERSION "1.0.0"
!define APP_EXE "BetterCPT.exe"
!define UNINST_EXE "uninstall.exe"

InstallDir "$PROFILE\AppData\Local\Programs\${APP_NAME}"

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "icons\icon.ico"
!define MUI_UNICON "icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_TEXT "创建桌面快捷方式"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Name "${APP_NAME}"
OutFile "target\release\bundle\nsis\${APP_NAME}-setup.exe"
RequestExecutionLevel user

; ── WebView2 detection ─────────────────────────────────────────
!define WEBVIEW2_REG_KEY "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define WEBVIEW2_REG_KEY_ALT "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

Function CreateDesktopShortcut
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
FunctionEnd

Section "Install"
  ; ── 1. WebView2 Runtime ────────────────────────────────────────
  DetailPrint "检查 WebView2 Runtime..."
  ReadRegStr $0 HKLM "${WEBVIEW2_REG_KEY}" "pv"
  IfErrors 0 webview2_done
  ReadRegStr $0 HKLM "${WEBVIEW2_REG_KEY_ALT}" "pv"
  IfErrors 0 webview2_done
    DetailPrint "安装 WebView2 Runtime..."
    SetOutPath "$TEMP"
    File "/oname=wv2setup.exe" "redist\MicrosoftEdgeWebview2Setup.exe"
    ExecWait '"$TEMP\wv2setup.exe" /silent /install' $1
    Delete "$TEMP\wv2setup.exe"
    DetailPrint "WebView2 安装完成 (exit=$1)"
  webview2_done:

  ; ── 2. Application files ───────────────────────────────────────
  SetOutPath "$INSTDIR"

  File "target\release\BetterCPT.exe"
  File "..\widget-qt\build\widget-qt.exe"
  File "..\widget-qt\build\*.dll"

  SetOutPath "$INSTDIR\platforms"
  File "..\widget-qt\build\platforms\*.dll"
  SetOutPath "$INSTDIR\styles"
  File "..\widget-qt\build\styles\*.dll"
  SetOutPath "$INSTDIR\imageformats"
  File "..\widget-qt\build\imageformats\*.dll"
  SetOutPath "$INSTDIR\iconengines"
  File "..\widget-qt\build\iconengines\*.dll"
  SetOutPath "$INSTDIR\generic"
  File "..\widget-qt\build\generic\*.dll"
  SetOutPath "$INSTDIR\networkinformation"
  File "..\widget-qt\build\networkinformation\*.dll"
  SetOutPath "$INSTDIR\tls"
  File "..\widget-qt\build\tls\*.dll"

  SetOutPath "$INSTDIR\frontend"
  File /r "frontend\*"

  SetOutPath "$INSTDIR\plugins"
  File /r "..\plugins\*"

  SetOutPath "$INSTDIR"

  WriteUninstaller "$INSTDIR\${UNINST_EXE}"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\${UNINST_EXE}"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" '"$INSTDIR\${UNINST_EXE}"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayIcon" '"$INSTDIR\${APP_EXE}"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "Publisher" "BetterCPT"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\widget-qt.exe"
  Delete "$INSTDIR\${UNINST_EXE}"
  Delete "$INSTDIR\*.dll"
  RMDir /r "$INSTDIR\platforms"
  RMDir /r "$INSTDIR\styles"
  RMDir /r "$INSTDIR\imageformats"
  RMDir /r "$INSTDIR\iconengines"
  RMDir /r "$INSTDIR\generic"
  RMDir /r "$INSTDIR\networkinformation"
  RMDir /r "$INSTDIR\tls"
  RMDir /r "$INSTDIR\frontend"
  RMDir /r "$INSTDIR\plugins"
  RMDir /r "$INSTDIR\log"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
SectionEnd
