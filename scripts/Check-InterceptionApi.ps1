param([string]$ResultPath)
$ErrorActionPreference = 'Stop'
# Independent ABI check, using the same public API. Never installs or injects input.
$libraryPath = Join-Path (Split-Path -Parent $PSScriptRoot) 'vendor\interception\v1.0.1\Interception\library\x64\interception.dll'
$libraryLiteral = $libraryPath.Replace('"', '""')
$source = @"
using System;
using System.Runtime.InteropServices;
public static class InterceptionApiCheck {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate int Predicate(int device);
    [DllImport(@"$libraryLiteral", CallingConvention=CallingConvention.Cdecl)] static extern IntPtr interception_create_context();
    [DllImport(@"$libraryLiteral", CallingConvention=CallingConvention.Cdecl)] static extern void interception_destroy_context(IntPtr context);
    [DllImport(@"$libraryLiteral", CallingConvention=CallingConvention.Cdecl)] static extern uint interception_get_hardware_id(IntPtr context, int device, [Out] byte[] buffer, uint size);
    [DllImport(@"$libraryLiteral", CallingConvention=CallingConvention.Cdecl)] static extern ushort interception_get_filter(IntPtr context, int device);
    [DllImport(@"$libraryLiteral", CallingConvention=CallingConvention.Cdecl)] static extern void interception_set_filter(IntPtr context, Predicate predicate, ushort filter);
    public static string Check() {
        var context = interception_create_context();
        if (context == IntPtr.Zero) throw new Exception("Context unavailable");
        int calls = 0, matches = 0, target = 0, count = 0;
        Predicate predicate = device => { calls++; if (device == target) { matches++; return 1; } return 0; };
        try {
            for (int device = 1; device <= 10; device++) {
                var bytes = new byte[8192];
                uint length = interception_get_hardware_id(context, device, bytes, (uint)bytes.Length);
                if (length > bytes.Length) throw new Exception("Invalid hardware ID length");
                string ids = System.Text.Encoding.Unicode.GetString(bytes, 0, (int)length).ToUpperInvariant();
                if (ids.Contains("VID_1532&PID_022B") && ids.Contains("MI_00")) { target = device; count++; }
            }
            if (count != 1) throw new Exception("Need exactly one V2 MI_00 slot");
            ushort before = interception_get_filter(context, target);
            interception_set_filter(context, predicate, 3);
            ushort after = interception_get_filter(context, target);
            interception_set_filter(context, predicate, 0);
            ushort cleared = interception_get_filter(context, target);
            return String.Format("slot={0} before={1} requested=3 after={2} cleared={3} predicateCalls={4} predicateMatches={5}", target, before, after, cleared, calls, matches);
        } finally {
            if (target != 0) interception_set_filter(context, predicate, 0);
            interception_destroy_context(context);
            GC.KeepAlive(predicate);
        }
    }
}
"@
Add-Type -TypeDefinition $source
$result = [InterceptionApiCheck]::Check()
if ($ResultPath) { $result | Set-Content -LiteralPath $ResultPath }
$result
