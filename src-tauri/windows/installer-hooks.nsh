; Sonata installer hooks (feature 012, research R16): put the bundled `sonata` command on the
; user's Path, and take it off again on uninstall. PowerShell edits the raw registry value (so
; entries such as %USERPROFILE%\bin stay unexpanded), then clears a dummy user variable, which
; makes Windows tell running programs that the environment changed.

!macro SONATA_SET_PATH ADD
  nsExec::ExecToLog `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$k = 'HKCU:\Environment'; $$d = '$INSTDIR\bin'; $$p = (Get-Item $$k).GetValue('Path', '', 'DoNotExpandEnvironmentNames'); $$parts = @($$p -split ';' | Where-Object { $$_ -and $$_ -ne $$d }); if (${ADD}) { $$parts = @($$d) + $$parts }; Set-ItemProperty -Path $$k -Name Path -Value ($$parts -join ';') -Type ExpandString; [Environment]::SetEnvironmentVariable('SONATA_PATH_REFRESH', $$null, 'User')"`
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro SONATA_SET_PATH "$$true"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro SONATA_SET_PATH "$$false"
!macroend
