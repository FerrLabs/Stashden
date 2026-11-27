param([Parameter(Mandatory)][string]$Path)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class PeResources
{
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr LoadLibraryEx(string file, IntPtr reserved, uint flags);

    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool FreeLibrary(IntPtr module);

    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);

    public static bool Has(string file, int type, int id)
    {
        IntPtr module = LoadLibraryEx(file, IntPtr.Zero, 0x22);
        if (module == IntPtr.Zero) throw new Win32Exception();
        try { return FindResource(module, (IntPtr)id, (IntPtr)type) != IntPtr.Zero; }
        finally { FreeLibrary(module); }
    }
}
'@

$exe = (Resolve-Path -LiteralPath $Path).Path
$failures = @()

if (-not [PeResources]::Has($exe, 14, 1)) {
    $failures += 'no RT_GROUP_ICON resource with id 1, the one GPUI and the shell load'
}
if (-not [PeResources]::Has($exe, 16, 1)) {
    $failures += 'no RT_VERSION resource'
}
$info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($exe)
if ($info.ProductName -cne 'Stashden') {
    $failures += "ProductName is '$($info.ProductName)', expected 'Stashden'"
}
if ($info.CompanyName -cne 'FerrLabs') {
    $failures += "CompanyName is '$($info.CompanyName)', expected 'FerrLabs'"
}
if (-not $info.FileVersion) {
    $failures += 'FileVersion is empty'
}

if ($failures.Count -gt 0) {
    foreach ($failure in $failures) { Write-Error "${exe}: $failure" -ErrorAction Continue }
    exit 1
}
Write-Host "$exe carries the icon and version information, FileVersion $($info.FileVersion)"
