' Focus Desktop hidden launcher (v1.10.2, #40).
' Double-click this file to start Focus without a console window.
' launch-focus.cmd (rebuild / monitor modes) is unchanged.
Dim fso, dir, shell
Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
Set shell = CreateObject("WScript.Shell")
shell.Run """" & dir & "\launch-focus.cmd""", 0, False
Set shell = Nothing
Set fso = Nothing