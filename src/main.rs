#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use serde::Deserialize;
use std::{process::Command, sync::mpsc, thread};
#[cfg(windows)] use std::os::windows::process::CommandExt;
#[derive(Clone, Deserialize, Default)]
#[serde(default)]
struct Device { id:String, name:String, service:String, inf:String, provider:String, version:String }
fn run(exe:&str,args:&[&str])->Result<String,String>{
 let mut c=Command::new(exe); c.args(args);
 #[cfg(windows)] c.creation_flags(0x08000000);
 let o=c.output().map_err(|e|e.to_string())?;
 let s=format!("{}{}",String::from_utf8_lossy(&o.stdout),String::from_utf8_lossy(&o.stderr));
 if o.status.success(){Ok(s)}else{Err(format!("{}: {}",o.status,s))}
}
fn ps(script:&str)->Result<String,String>{run("powershell.exe",&["-NoProfile","-NonInteractive","-Command",script])}
fn scan()->Result<Vec<Device>,String>{
 let s=ps(r#"$ErrorActionPreference='Stop'; $a=@(Get-PnpDevice -PresentOnly | Where-Object {$_.InstanceId -match '^USB\\VID_0483&PID_3748\\'} | ForEach-Object {$d=$_; function prop($k){try{[string](Get-PnpDeviceProperty -InstanceId $d.InstanceId -KeyName $k -ErrorAction Stop).Data}catch{''}}; [pscustomobject]@{id=$d.InstanceId;name=$d.FriendlyName;service=(prop 'DEVPKEY_Device_Service');inf=(prop 'DEVPKEY_Device_DriverInfPath');provider=(prop 'DEVPKEY_Device_DriverProvider');version=(prop 'DEVPKEY_Device_DriverVersion')}}); ConvertTo-Json -InputObject $a -Compress"#)?;
 serde_json::from_str(s.trim()).map_err(|e|format!("{}: {}",e,s))
}
fn quote(s:&str)->String{format!("'{}'",s.replace('\'',"''"))}
// Use SetupAPI through a small in-process C# interop assembly hosted by Windows PowerShell.
// DiInstallDevice selects a specific driver node for the selected instance, not every matching device.
const NATIVE:&str=include_str!("driver.cs");
fn native(id:&str, inf:&str, apply:bool)->Result<String,String>{
 let script=format!("$ErrorActionPreference='Stop'; Add-Type -TypeDefinition @'\n{}\n'@; [DriverOps]::Run({}, {}, ${})",NATIVE,quote(id),quote(inf),if apply{"true"}else{"false"}); ps(&script)
}
enum ResultMsg { Scan(Result<Vec<Device>,String>), Log(Result<String,String>) }
struct App { devices:Vec<Device>, selected:usize, log:String, inf:String, guid:String, confirm:bool, pending:Option<mpsc::Receiver<ResultMsg>> }
impl Default for App {fn default()->Self{let mut a=Self{devices:vec![],selected:0,log:String::new(),inf:"winusb.inf".into(),guid:String::new(),confirm:false,pending:None};a.refresh();a}}
impl App {fn job(&mut self,f:impl FnOnce()->ResultMsg+Send+'static){let(tx,rx)=mpsc::channel();self.pending=Some(rx);thread::spawn(move||{let _=tx.send(f());});}fn refresh(&mut self){self.job(||ResultMsg::Scan(scan()));}}
impl eframe::App for App {fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){
 if let Some(rx)=&self.pending {if let Ok(msg)=rx.try_recv(){self.pending=None;match msg{ResultMsg::Scan(Ok(d))=>{self.devices=d;self.selected=0;self.log.push_str("\nScan complete.\n");},ResultMsg::Scan(Err(e))|ResultMsg::Log(Err(e))=>self.log.push_str(&format!("\nERROR: {e}\n")),ResultMsg::Log(Ok(s))=>self.log.push_str(&format!("\n{s}\n"))}}else{ctx.request_repaint_after(std::time::Duration::from_millis(100));}}
 egui::CentralPanel::default().show(ctx,|ui|{ui.heading("KTYLabs ST-Link V2 Driver Manager ARM64");ui.label("v0.1.0 • Windows x64 / ARM64 • ST-LINK/V2 0483:3748 only");ui.separator();let busy=self.pending.is_some();ui.add_enabled_ui(!busy,|ui|{
 if ui.button("Refresh devices").clicked(){self.confirm=false;self.refresh();}
 for (i,d) in self.devices.iter().enumerate(){if ui.selectable_label(self.selected==i,format!("{} | {}",d.name,d.id)).clicked(){self.selected=i;self.confirm=false;}}
 if let Some(d)=self.devices.get(self.selected).cloned(){ui.separator();ui.label(format!("Service: {} | INF: {}",d.service,d.inf));ui.label(format!("Provider: {} | Version: {}",d.provider,d.version));
 ui.horizontal(|ui|{if ui.button("List driver candidates").clicked(){let id=d.id.clone();self.job(move||ResultMsg::Log(native(&id,"",false)));}if ui.button("Select WinUSB").clicked(){self.inf="winusb.inf".into();self.confirm=false;}if ui.button("Use current INF").clicked(){self.inf=d.inf.clone();self.confirm=false;}});
 ui.horizontal(|ui|{ui.label("Target INF (exact name from candidate list)");if ui.text_edit_singleline(&mut self.inf).changed(){self.confirm=false;}});
 if ui.button("Stage signed INF package...").clicked(){if let Some(p)=rfd::FileDialog::new().add_filter("INF",&["inf"]).pick_file(){self.job(move||ResultMsg::Log(run("pnputil.exe",&["/add-driver",&p.to_string_lossy()])));}}
 ui.checkbox(&mut self.confirm,"I closed debugger tools and recorded the original INF for rollback.");
 if ui.add_enabled(self.confirm && !self.inf.trim().is_empty(),egui::Button::new("Apply selected driver")).clicked(){let id=d.id.clone();let inf=self.inf.trim().to_owned();self.confirm=false;self.job(move||ResultMsg::Log(native(&id,&inf,true)));}
 ui.horizontal(|ui|{ui.label("Optional interface GUID");ui.text_edit_singleline(&mut self.guid);});
 if ui.add_enabled(d.service.eq_ignore_ascii_case("WinUSB") && !self.guid.is_empty(),egui::Button::new("Add GUID (preserve existing)")).clicked(){let id=d.id.clone();let g=self.guid.clone();self.job(move||ResultMsg::Log(ps(&format!(r#"$ErrorActionPreference='Stop'; $g=[guid]::Parse({}); $p='HKLM:\SYSTEM\CurrentControlSet\Enum\'+{}+'\Device Parameters'; $old=@((Get-ItemProperty -LiteralPath $p -Name DeviceInterfaceGUIDs -ErrorAction SilentlyContinue).DeviceInterfaceGUIDs | Where-Object {{$_}}); $v=@($old + $g.ToString('B') | Select-Object -Unique); New-ItemProperty -LiteralPath $p -Name DeviceInterfaceGUIDs -PropertyType MultiString -Value $v -Force | Out-String"#,quote(&g),quote(&id)))));}
 if ui.button("Restart selected device").clicked(){let id=d.id.clone();self.job(move||ResultMsg::Log(run("pnputil.exe",&["/restart-device",&id])));}
 if ui.button("Verify device / interfaces").clicked(){let id=d.id.clone();self.job(move||ResultMsg::Log(run("pnputil.exe",&["/enum-devices","/instanceid",&id,"/interfaces","/drivers"])));}
 }else{ui.label("Connect an ST-LINK/V2 and refresh.");}});if busy{ui.spinner();}ui.separator();ui.horizontal(|ui|{ui.heading("Operation log");if ui.button("Copy").clicked(){ctx.copy_text(self.log.clone());}});egui::ScrollArea::vertical().stick_to_bottom(true).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut self.log).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(12));});});}}
fn main()->eframe::Result {eframe::run_native("ST-Link V2 Driver Manager ARM64",eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([850.,700.]),..Default::default()},Box::new(|_|Ok(Box::new(App::default()))))}
