using System;
using System.Runtime.InteropServices;
using System.Text;
using System.ComponentModel;
using System.IO;
public static class DriverOps {
 [StructLayout(LayoutKind.Sequential)] struct Dev {public uint cb; public Guid cls; public uint inst; public IntPtr reserved;}
 [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct Drv {public uint cb,type; public IntPtr reserved; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=256)] public string desc; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=256)] public string manufacturer; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=256)] public string provider; public System.Runtime.InteropServices.ComTypes.FILETIME date; public ulong version;}
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Detail {public uint cb; public System.Runtime.InteropServices.ComTypes.FILETIME date; public uint offset,length; public IntPtr reserved; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=256)] public string section; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=260)] public string inf; [MarshalAs(UnmanagedType.ByValTStr,SizeConst=256)] public string desc; public ushort hardware;}
 [DllImport("setupapi.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern IntPtr SetupDiGetClassDevsW(IntPtr cls,string enumerator,IntPtr hwnd,uint flags);
 [DllImport("setupapi.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool SetupDiOpenDeviceInfoW(IntPtr set,string id,IntPtr hwnd,uint flags,ref Dev dev);
 [DllImport("setupapi.dll",SetLastError=true)] static extern bool SetupDiBuildDriverInfoList(IntPtr set,ref Dev dev,uint type);
 [DllImport("setupapi.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool SetupDiEnumDriverInfoW(IntPtr set,ref Dev dev,uint type,uint index,ref Drv drv);
 [DllImport("setupapi.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool SetupDiGetDriverInfoDetailW(IntPtr set,ref Dev dev,ref Drv drv,IntPtr detail,uint size,out uint required);
 [DllImport("setupapi.dll",SetLastError=true)] static extern bool SetupDiDestroyDriverInfoList(IntPtr set,ref Dev dev,uint type);
 [DllImport("setupapi.dll",SetLastError=true)] static extern bool SetupDiDestroyDeviceInfoList(IntPtr set);
 [DllImport("newdev.dll",SetLastError=true)] static extern bool DiInstallDevice(IntPtr hwnd,IntPtr set,ref Dev dev,ref Drv drv,uint flags,[MarshalAs(UnmanagedType.Bool)] out bool reboot);
 static void Check(bool ok){if(!ok)throw new Win32Exception(Marshal.GetLastWin32Error());}
 public static string Run(string id,string inf,bool apply){
 if(!id.StartsWith(@"USB\VID_0483&PID_3748\",StringComparison.OrdinalIgnoreCase))throw new Exception("Unsupported device");
 IntPtr set=SetupDiGetClassDevsW(IntPtr.Zero,null,IntPtr.Zero,6); if(set==new IntPtr(-1))throw new Win32Exception(Marshal.GetLastWin32Error());
 Dev dev=new Dev();dev.cb=(uint)Marshal.SizeOf(typeof(Dev));bool built=false;var result=new StringBuilder();
 try{Check(SetupDiOpenDeviceInfoW(set,id,IntPtr.Zero,0,ref dev));Check(SetupDiBuildDriverInfoList(set,ref dev,2));built=true;
 for(uint i=0;;i++){Drv drv=new Drv();drv.cb=(uint)Marshal.SizeOf(typeof(Drv));if(!SetupDiEnumDriverInfoW(set,ref dev,2,i,ref drv)){int err=Marshal.GetLastWin32Error();if(err==259)break;throw new Win32Exception(err);}
 uint needed;SetupDiGetDriverInfoDetailW(set,ref dev,ref drv,IntPtr.Zero,0,out needed);if(needed==0)throw new Win32Exception(Marshal.GetLastWin32Error());uint bytes=Math.Max(needed,(uint)Marshal.SizeOf(typeof(Detail)));IntPtr p=Marshal.AllocHGlobal((int)bytes);
 try{Marshal.WriteInt32(p,Marshal.SizeOf(typeof(Detail)));Check(SetupDiGetDriverInfoDetailW(set,ref dev,ref drv,p,bytes,out needed));Detail detail=(Detail)Marshal.PtrToStructure(p,typeof(Detail));string name=Path.GetFileName(detail.inf);result.AppendLine(name+" | "+drv.desc+" | "+drv.provider+" | "+detail.inf);
 if(apply && String.Equals(name,inf,StringComparison.OrdinalIgnoreCase)){bool reboot;Check(DiInstallDevice(IntPtr.Zero,set,ref dev,ref drv,0,out reboot));return result.ToString()+"Installation API succeeded. Reboot required: "+reboot+". Refresh and verify service.";}}
 finally{Marshal.FreeHGlobal(p);}}
 if(apply)throw new Exception("No compatible candidate with that INF. Stage a signed compatible package and retry.\n"+result);return result.Length==0?"No compatible candidates found.":result.ToString();}
 finally{if(built)SetupDiDestroyDriverInfoList(set,ref dev,2);SetupDiDestroyDeviceInfoList(set);}}
}
