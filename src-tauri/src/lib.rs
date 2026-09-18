use futures_util::StreamExt;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, path::{Path, PathBuf}, process::Command, time::Duration};

const SERVICE: &str = "QA Lab Launcher";
const ACCOUNT: &str = "github-oauth-token";
const API: &str = "https://api.github.com";

type Result<T> = std::result::Result<T, String>;

#[derive(Serialize, Deserialize, Clone, Debug)] pub struct DeviceCode { device_code:String, user_code:String, verification_uri:String, expires_in:u64, interval:u64 }
#[derive(Serialize, Deserialize, Clone, Debug)] struct Manifest { name:String, #[serde(default)] description:String, entrypoint:String, #[serde(default)] icon:String, #[serde(default)] asset:AssetRule }
#[derive(Serialize, Deserialize, Clone, Debug, Default)] struct AssetRule { #[serde(default)] pattern:String, #[serde(default)] sha256_asset:String }
#[derive(Serialize, Deserialize, Clone, Debug)] struct Repo { name:String, owner:Owner, default_branch:String }
#[derive(Serialize, Deserialize, Clone, Debug)] struct Owner { login:String }
#[derive(Serialize, Deserialize, Clone, Debug)] struct Release { tag_name:String, assets:Vec<ReleaseAsset> }
#[derive(Serialize, Deserialize, Clone, Debug)] struct ReleaseAsset { name:String, browser_download_url:String }
#[derive(Serialize, Deserialize, Clone, Debug)] pub struct Tool { owner:String, repo:String, name:String, description:String, icon_url:Option<String>, remote_version:String, local_version:Option<String>, status:String, asset_name:String }
#[derive(Serialize)] struct Catalog { tools:Vec<Tool>, unconfigured:usize, warnings:Vec<String> }
#[derive(Serialize, Deserialize, Default)] struct Registry { tools:std::collections::HashMap<String,Installed> }
#[derive(Serialize, Deserialize, Clone)] struct Installed { version:String, entrypoint:String, installed_at:String }

fn client(token:Option<&str>)->Result<Client>{
 let mut h=header::HeaderMap::new(); h.insert(header::USER_AGENT,"QA-Lab-Launcher/0.1".parse().unwrap()); h.insert(header::ACCEPT,"application/vnd.github+json".parse().unwrap()); h.insert("X-GitHub-Api-Version","2022-11-28".parse().unwrap());
 if let Some(t)=token { h.insert(header::AUTHORIZATION,format!("Bearer {t}").parse().map_err(|_|"Token non valido")?); }
 Client::builder().default_headers(h).build().map_err(err)
}
fn err<E:std::fmt::Display>(e:E)->String{e.to_string()}
fn client_id()->Result<String>{option_env!("GITHUB_CLIENT_ID").map(str::to_owned).or_else(||std::env::var("GITHUB_CLIENT_ID").ok()).filter(|v|!v.trim().is_empty()).ok_or_else(||"GitHub OAuth Client ID non configurato. Imposta GITHUB_CLIENT_ID prima della build.".into())}
fn entry()->Result<keyring::Entry>{keyring::Entry::new(SERVICE,ACCOUNT).map_err(err)}
fn token()->Result<String>{entry()?.get_password().map_err(|_|"Sessione GitHub non disponibile. Accedi di nuovo.".into())}
fn root()->Result<PathBuf>{dirs::data_local_dir().map(|p|p.join("QALab")).ok_or_else(||"Impossibile trovare LOCALAPPDATA".into())}
fn registry_path()->Result<PathBuf>{Ok(root()?.join("registry.json"))}
fn read_registry()->Registry{registry_path().ok().and_then(|p|fs::read(p).ok()).and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default()}
fn write_registry(r:&Registry)->Result<()>{let p=registry_path()?;fs::create_dir_all(p.parent().unwrap()).map_err(err)?;let tmp=p.with_extension("tmp");fs::write(&tmp,serde_json::to_vec_pretty(r).map_err(err)?).map_err(err)?;fs::rename(tmp,p).map_err(err)}
fn safe_segment(s:&str)->bool{!s.is_empty()&&s.chars().all(|c|c.is_ascii_alphanumeric()||"-_.".contains(c))&&s!="."&&s!=".."}
fn normalize_tag(s:&str)->&str{s.strip_prefix('v').unwrap_or(s)}

#[tauri::command]
fn has_session()->bool{entry().and_then(|e|e.get_password().map_err(err)).is_ok()}
#[tauri::command]
async fn begin_login()->Result<DeviceCode>{let id=client_id()?;let res=client(None)?.post("https://github.com/login/device/code").form(&[("client_id",id.as_str()),("scope","repo read:user")]).send().await.map_err(err)?;if !res.status().is_success(){return Err(format!("GitHub ha rifiutato l'avvio del login ({})",res.status()))}res.json().await.map_err(err)}
#[tauri::command]
async fn finish_login(device_code:String,interval:u64,expires_in:u64)->Result<()>{let c=client(None)?;let id=client_id()?;let attempts=(expires_in/(interval.max(1))).max(1);let mut wait=interval.max(1);for _ in 0..attempts{tokio::time::sleep(Duration::from_secs(wait)).await;let v:serde_json::Value=c.post("https://github.com/login/oauth/access_token").form(&[("client_id",id.as_str()),("device_code",device_code.as_str()),("grant_type","urn:ietf:params:oauth:grant-type:device_code")]).send().await.map_err(err)?.json().await.map_err(err)?;if let Some(t)=v.get("access_token").and_then(|x|x.as_str()){entry()?.set_password(t).map_err(err)?;return Ok(())}match v.get("error").and_then(|x|x.as_str()){Some("authorization_pending")=>{},Some("slow_down")=>wait+=5,Some("access_denied")=>return Err("Accesso annullato su GitHub.".into()),Some("expired_token")=>return Err("Il codice è scaduto. Riprova.".into()),Some(e)=>return Err(format!("Login GitHub non riuscito: {e}")),None=>{}}}Err("Tempo scaduto. Avvia nuovamente l'accesso.".into())}
#[tauri::command]
fn logout()->Result<()>{match entry()?.delete_credential(){Ok(_)=>Ok(()),Err(keyring::Error::NoEntry)=>Ok(()),Err(e)=>Err(err(e))}}

async fn github_json<T:for<'a>Deserialize<'a>>(c:&Client,url:&str)->Result<Option<T>>{let r=c.get(url).send().await.map_err(err)?;if r.status()==reqwest::StatusCode::NOT_FOUND{return Ok(None)}if !r.status().is_success(){return Err(format!("GitHub API: {} per {}",r.status(),url))}Ok(Some(r.json().await.map_err(err)?))}
fn choose_asset<'a>(m:&Manifest,assets:&'a[ReleaseAsset])->Option<&'a ReleaseAsset>{let pattern=if m.asset.pattern.is_empty(){"-win-x64.zip"}else{&m.asset.pattern};assets.iter().find(|a|a.name.to_lowercase().ends_with(&pattern.to_lowercase())&&a.name.to_lowercase().ends_with(".zip"))}
#[tauri::command]
async fn load_catalog()->Result<Catalog>{
 let tok=token()?;let c=client(Some(&tok))?;let mut repos=Vec::new();for page in 1..=20{let url=format!("{API}/user/repos?visibility=all&affiliation=owner,collaborator,organization_member&sort=full_name&per_page=100&page={page}");let batch:Vec<Repo>=github_json(&c,&url).await?.unwrap_or_default();let n=batch.len();repos.extend(batch);if n<100{break}}
 let reg=read_registry();let mut tools=Vec::new();let mut warnings=Vec::new();let mut unconfigured=0;
 for repo in repos {let base=format!("{API}/repos/{}/{}",repo.owner.login,repo.name);let content:Option<serde_json::Value>=github_json(&c,&format!("{base}/contents/launcher.json?ref={}",repo.default_branch)).await?;let Some(meta)=content else{unconfigured+=1;continue};let Some(dl)=meta.get("download_url").and_then(|v|v.as_str())else{unconfigured+=1;continue};let manifest=match c.get(dl).send().await.and_then(|r|r.error_for_status()).map_err(err){Ok(r)=>match r.json::<Manifest>().await{Ok(m) if !m.name.trim().is_empty()&&!m.entrypoint.trim().is_empty()=>m,_=>{warnings.push(format!("{}/{}: launcher.json non valido",repo.owner.login,repo.name));continue}},Err(e)=>{warnings.push(format!("{}/{}: {e}",repo.owner.login,repo.name));continue}};let release:Option<Release>=github_json(&c,&format!("{base}/releases/latest")).await?;let Some(release)=release else{warnings.push(format!("{}/{}: nessuna latest release",repo.owner.login,repo.name));continue};let Some(asset)=choose_asset(&manifest,&release.assets)else{warnings.push(format!("{}/{}: asset Windows ZIP non trovato",repo.owner.login,repo.name));continue};let key=format!("{}/{}",repo.owner.login,repo.name);let local=reg.tools.get(&key).map(|i|i.version.clone());let status=match &local{None=>"not_installed",Some(v) if normalize_tag(v)==normalize_tag(&release.tag_name)=>"updated",Some(_)=>"update_available"}.into();let icon_url=if manifest.icon.is_empty(){None}else{Some(format!("https://raw.githubusercontent.com/{}/{}/{}/{}",repo.owner.login,repo.name,repo.default_branch,manifest.icon))};tools.push(Tool{owner:repo.owner.login,repo:repo.name,name:manifest.name,description:manifest.description,icon_url,remote_version:release.tag_name,local_version:local,status,asset_name:asset.name.clone()}) }
 tools.sort_by(|a,b|a.name.to_lowercase().cmp(&b.name.to_lowercase()));Ok(Catalog{tools,unconfigured,warnings})
}

fn extract_zip(zip_path:&Path,dest:&Path)->Result<()>{let f=fs::File::open(zip_path).map_err(err)?;let mut z=zip::ZipArchive::new(f).map_err(err)?;for i in 0..z.len(){let mut file=z.by_index(i).map_err(err)?;let Some(rel)=file.enclosed_name()else{return Err("Archivio non sicuro: percorso non valido".into())};let out=dest.join(rel);if file.is_dir(){fs::create_dir_all(&out).map_err(err)?}else{if let Some(p)=out.parent(){fs::create_dir_all(p).map_err(err)?}let mut target=fs::File::create(out).map_err(err)?;io::copy(&mut file,&mut target).map_err(err)?;}}Ok(())}
async fn download(c:&Client,url:&str,path:&Path)->Result<()> {let r=c.get(url).send().await.map_err(err)?.error_for_status().map_err(err)?;let mut f=tokio::fs::File::create(path).await.map_err(err)?;let mut stream=r.bytes_stream();use tokio::io::AsyncWriteExt;while let Some(chunk)=stream.next().await{f.write_all(&chunk.map_err(err)?).await.map_err(err)?}f.flush().await.map_err(err)}
#[tauri::command]
async fn install_tool(tool:Tool)->Result<()>{if !safe_segment(&tool.owner)||!safe_segment(&tool.repo)||!safe_segment(&tool.remote_version){return Err("Nome repository o versione non sicuri".into())}let tok=token()?;let c=client(Some(&tok))?;let base=format!("{API}/repos/{}/{}",tool.owner,tool.repo);let release:Release=github_json(&c,&format!("{base}/releases/latest")).await?.ok_or("Release non trovata")?;if release.tag_name!=tool.remote_version{return Err("È disponibile una release più recente. Aggiorna il catalogo.".into())}let asset=release.assets.iter().find(|a|a.name==tool.asset_name).ok_or("Asset non più disponibile")?;let manifest_meta:serde_json::Value=github_json(&c,&format!("{base}/contents/launcher.json")).await?.ok_or("Manifest non trovato")?;let dl=manifest_meta.get("download_url").and_then(|v|v.as_str()).ok_or("Manifest non scaricabile")?;let manifest:Manifest=c.get(dl).send().await.map_err(err)?.error_for_status().map_err(err)?.json().await.map_err(err)?;
 let dest=root()?.join("tools").join(&tool.repo).join(&tool.remote_version);fs::create_dir_all(&dest).map_err(err)?;let archive=dest.join(".download.zip");if let Err(e)=download(&c,&asset.browser_download_url,&archive).await{let _=fs::remove_file(&archive);return Err(format!("Download non riuscito: {e}"))}if !manifest.asset.sha256_asset.is_empty(){let checksum_asset=release.assets.iter().find(|a|a.name==manifest.asset.sha256_asset).ok_or("Asset checksum dichiarato ma assente")?;let checksum_path=dest.join(".sha256");download(&c,&checksum_asset.browser_download_url,&checksum_path).await?;let expected=fs::read_to_string(&checksum_path).map_err(err)?.split_whitespace().next().unwrap_or("").to_lowercase();let actual=hex::encode(Sha256::digest(fs::read(&archive).map_err(err)?));if expected!=actual{return Err("Verifica SHA-256 fallita: il pacchetto potrebbe essere danneggiato.".into())}}
 extract_zip(&archive,&dest)?;let _=fs::remove_file(archive);let exe=dest.join(&manifest.entrypoint);if !exe.is_file(){return Err(format!("Installazione incompleta: {} non trovato nel pacchetto",manifest.entrypoint))}let mut reg=read_registry();reg.tools.insert(format!("{}/{}",tool.owner,tool.repo),Installed{version:tool.remote_version,entrypoint:manifest.entrypoint,installed_at:chrono::Utc::now().to_rfc3339()});write_registry(&reg)}
#[tauri::command]
fn open_tool(tool:Tool)->Result<()>{let reg=read_registry();let i=reg.tools.get(&format!("{}/{}",tool.owner,tool.repo)).ok_or("Tool non installato")?;let exe=root()?.join("tools").join(&tool.repo).join(&i.version).join(&i.entrypoint);if !exe.is_file(){return Err("Entrypoint non trovato. Reinstalla lo strumento.".into())}let cwd=exe.parent().unwrap();let ext=exe.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();let mut command=match ext.as_str(){"cmd"|"bat"=>{let mut c=Command::new("cmd.exe");c.arg("/d").arg("/c").arg(&exe);c},"ps1"=>{let mut c=Command::new("powershell.exe");c.arg("-NoProfile").arg("-File").arg(&exe);c},_=>Command::new(&exe)};command.current_dir(cwd).spawn().map_err(|e|format!("Impossibile avviare lo strumento: {e}"))?;Ok(())}
#[tauri::command]
fn open_url(url:String)->Result<()>{let parsed=url::Url::parse(&url).map_err(err)?;if parsed.scheme()!="https"||parsed.host_str()!=Some("github.com"){return Err("URL esterno non consentito".into())}open::that(url).map_err(err)}

pub fn run(){let log_dir=root().unwrap_or_else(|_|PathBuf::from("." )).join("logs");let _=fs::create_dir_all(&log_dir);let log=fs::OpenOptions::new().create(true).append(true).open(log_dir.join("launcher.log"));if let Ok(file)=log{let _=tracing_subscriber::fmt().with_writer(std::sync::Mutex::new(file)).try_init();}tauri::Builder::default().plugin(tauri_plugin_updater::Builder::new().build()).invoke_handler(tauri::generate_handler![has_session,begin_login,finish_login,logout,load_catalog,install_tool,open_tool,open_url]).run(tauri::generate_context!()).expect("error while running QA Lab Launcher")}

#[cfg(test)] mod tests{use super::*;#[test]fn asset_selection(){let m=Manifest{name:"x".into(),description:"".into(),entrypoint:"x.exe".into(),icon:"".into(),asset:AssetRule::default()};let a=vec![ReleaseAsset{name:"x-linux.zip".into(),browser_download_url:"".into()},ReleaseAsset{name:"tool-win-x64.zip".into(),browser_download_url:"".into()}];assert_eq!(choose_asset(&m,&a).unwrap().name,"tool-win-x64.zip")}#[test]fn path_segments(){assert!(safe_segment("tool-one"));assert!(!safe_segment("../evil"));assert!(!safe_segment("a/b"))}}
