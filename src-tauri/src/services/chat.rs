use super::{binaries::BinaryPaths, chat_edit::EditMap, media};
use crate::models::DownloadJob;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::{HashMap, HashSet}, fs, io::{BufRead, BufReader}, path::{Path, PathBuf}, process::Stdio, sync::{Arc, Mutex, OnceLock, atomic::{AtomicBool, Ordering}}};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Message { #[serde(default)] pub runs:Vec<MessageRun>, pub id:String, pub time:f64, pub author:String, pub author_id:String, pub text:String, pub kind:String, pub amount:Option<String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRun { pub text:String, pub image:Option<String> }
pub fn image_url(v:&Value)->Option<String>{
    v.get("thumbnails")?.as_array()?.iter().rev().filter_map(|t|t.get("url")?.as_str()).find(|s|valid_image_url(s)).map(str::to_owned)
}
pub fn valid_image_url(s:&str)->bool{
    url::Url::parse(s).ok().is_some_and(|u|u.scheme()=="https" && u.port_or_known_default()==Some(443) && u.username().is_empty() && u.password().is_none() && u.host_str().is_some_and(|h|["ytimg.com","ggpht.com","googleusercontent.com","youtube.com","fonts.gstatic.com"].iter().any(|d|h==*d||h.ends_with(&format!(".{d}")))))
}
fn rich_runs(v:&Value)->Vec<MessageRun>{
    if let Some(s)=v.get("simpleText").and_then(Value::as_str){return vec![MessageRun{text:s.into(),image:None}]}
    v.get("runs").and_then(Value::as_array).map(|runs|runs.iter().map(|r|{
        let text=rich(&serde_json::json!({"runs":[r]}));
        MessageRun{text,image:r.get("emoji").and_then(|e|image_url(&e["image"]))}
    }).collect()).unwrap_or_default()
}
#[derive(Clone, Debug, Serialize)]
pub struct Peak { pub start:f64, pub end:f64, pub count:usize }
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ChatReport {
    pub title:String, pub duration:f64, pub messages:Vec<Message>, pub normal:usize, pub super_chat:usize, pub super_sticker:usize,
    pub memberships:usize, pub participants:usize, pub untimed:usize, pub malformed:usize, pub peaks:Vec<Peak>, pub activity:Vec<Peak>,
    pub raw_path:String, pub edit:Option<EditMap>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ChatStatus { pub job_id:String, pub stage:String, pub message:String, pub percent:Option<f64> }
static OPERATIONS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
pub struct Operation { key:String, resource:Option<String>, pub cancel:Arc<AtomicBool> }
impl Operation {
    pub fn begin(job_id:&str)->Result<Self,String> { Self::begin_resource(job_id,None) }
    fn begin_resource(job_id:&str,resource:Option<String>)->Result<Self,String> {
        let mut map=OPERATIONS.get_or_init(Default::default).lock().unwrap();
        if map.contains_key(job_id) || resource.as_ref().is_some_and(|r|map.contains_key(r)) { return Err("Bu kayıt için bir sohbet işlemi zaten çalışıyor.".into()) }
        let cancel=Arc::new(AtomicBool::new(false));map.insert(job_id.into(),cancel.clone());if let Some(r)=&resource{map.insert(r.clone(),cancel.clone());}Ok(Self{key:job_id.into(),resource,cancel})
    }
}
impl Drop for Operation { fn drop(&mut self){let mut map=OPERATIONS.get().unwrap().lock().unwrap();map.remove(&self.key);if let Some(r)=&self.resource{map.remove(r);}} }
pub fn cancel(job_id:&str){if let Some(token)=OPERATIONS.get_or_init(Default::default).lock().unwrap().get(job_id){token.store(true,Ordering::Relaxed)}}
pub fn folder(job:&DownloadJob)->PathBuf { PathBuf::from(media::inspect(job).folder) }
pub fn base(job:&DownloadJob)->String { format!("chat-{}",job.request.item.id.chars().filter(|c|c.is_ascii_alphanumeric()||*c=='-'||*c=='_').collect::<String>()) }
pub fn raw_path(job:&DownloadJob)->PathBuf{folder(job).join(format!("{}.raw.jsonl",base(job)))}
pub fn edit_path(job:&DownloadJob)->PathBuf{folder(job).join(format!("{}.resolve.json",base(job)))}
pub fn status(app:&AppHandle,job:&DownloadJob,stage:&str,message:&str,percent:Option<f64>){
    let value=ChatStatus{job_id:job.job_id.clone(),stage:stage.into(),message:message.into(),percent};
    if percent.is_none(){let _=fs::write(folder(job).join(format!("{}.status.json",base(job))),serde_json::to_vec_pretty(&value).unwrap());}
    let _=app.emit("chat-progress",value);
}
pub fn saved_status(job:&DownloadJob)->Option<ChatStatus>{
    let path=folder(job).join(format!("{}.status.json",base(job)));
    let mut value:ChatStatus=serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    if matches!(value.stage.as_str(),"downloading"|"rendering")&&!OPERATIONS.get_or_init(Default::default).lock().unwrap().contains_key(&job.job_id){value.stage="interrupted".into();value.message="Önceki işlem kesildi; yeniden deneyebilirsiniz.".into()}
    Some(value)
}
fn rich(v:&Value)->String {
    if let Some(s)=v.get("simpleText").and_then(Value::as_str){return s.into()}
    v.get("runs").and_then(Value::as_array).map(|runs| runs.iter().map(|r|{
        r.get("text").and_then(Value::as_str).map(str::to_owned).or_else(||{
            let e=r.get("emoji")?;e.get("shortcuts").and_then(Value::as_array).and_then(|a|a.first()).and_then(Value::as_str)
                .or_else(||e.pointer("/image/accessibility/accessibilityData/label").and_then(Value::as_str)).map(str::to_owned)
        }).unwrap_or_default()
    }).collect()).unwrap_or_default()
}
fn walk(v:&Value,offset:Option<f64>,out:&mut Vec<Message>,seen:&mut HashSet<String>,untimed:&mut usize){
    match v {
        Value::Array(a)=>for value in a{walk(value,offset,out,seen,untimed)},
        Value::Object(obj)=>{
            let offset=obj.get("videoOffsetTimeMsec").and_then(|n|n.as_str().and_then(|s|s.parse::<f64>().ok()).or_else(||n.as_f64())).map(|n|n/1000.0).or(offset);
            for (key,value) in obj {
                let kind=match key.as_str(){"liveChatTextMessageRenderer"=>"normal","liveChatPaidMessageRenderer"=>"superchat","liveChatPaidStickerRenderer"=>"supersticker","liveChatMembershipItemRenderer"=>"membership",_=>{walk(value,offset,out,seen,untimed);continue}};
                let id=value.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                let Some(time)=offset.filter(|t|t.is_finite()&&*t>=0.0)else{*untimed+=1;continue};
                if !id.is_empty()&&!seen.insert(id.clone()){continue}
                let mut runs=rich_runs(&value["message"]);
                if runs.is_empty(){runs=rich_runs(&value["headerSubtext"])}
                if kind=="supersticker"{if let Some(image)=image_url(&value["sticker"]){runs=vec![MessageRun{text:value.pointer("/sticker/accessibility/accessibilityData/label").and_then(Value::as_str).unwrap_or("Super Sticker").into(),image:Some(image)}]}}
                let mut body=rich(&value["message"]);
                if body.is_empty(){body=rich(&value["headerSubtext"])}
                if body.is_empty()&&kind=="supersticker"{body=value.pointer("/sticker/accessibility/accessibilityData/label").and_then(Value::as_str).unwrap_or("Super Sticker").into()}
                let amount=rich(&value["purchaseAmountText"]);
                out.push(Message{runs,id,time,author:rich(&value["authorName"]),author_id:value.get("authorExternalChannelId").and_then(Value::as_str).unwrap_or("").into(),text:body,kind:kind.into(),amount:if amount.is_empty(){None}else{Some(amount)}});
            }
        },_=>{}
    }
}
pub fn parse(path:&Path)->Result<(Vec<Message>,usize,usize),String>{
    let file=fs::File::open(path).map_err(|e|e.to_string())?;
    let mut out=Vec::new();let mut seen=HashSet::new();let(mut untimed,mut malformed)=(0,0);
    for line in BufReader::new(file).lines(){let line=line.map_err(|e|e.to_string())?;if line.trim().is_empty(){continue}
        match serde_json::from_str::<Value>(&line){Ok(v)=>walk(&v,None,&mut out,&mut seen,&mut untimed),Err(_)=>malformed+=1}
    }
    out.sort_by(|a,b|a.time.total_cmp(&b.time));Ok((out,untimed,malformed))
}
pub fn analyze(messages:&[Message])->(Vec<Peak>,Vec<Peak>){
    let duration=messages.last().map(|m|m.time.ceil() as usize+1).unwrap_or(0).min(7*86400);
    let(mut left,mut right)=(0,0);let mut windows=Vec::new();
    for second in 0..duration {let start=second as f64;while left<messages.len()&&messages[left].time<start{left+=1}right=right.max(left);while right<messages.len()&&messages[right].time<start+15.0{right+=1}if right>left{windows.push(Peak{start,end:start+15.0,count:right-left})}}
    let stride=(duration/300).max(1);let activity=windows.iter().filter(|w|w.start as usize%stride==0).cloned().collect();
    windows.sort_by(|a,b|b.count.cmp(&a.count).then_with(||a.start.total_cmp(&b.start)));
    let mut peaks:Vec<Peak>=Vec::new();for peak in windows{if peaks.iter().all(|p|peak.start>=p.end||peak.end<=p.start){peaks.push(peak);if peaks.len()==50{break}}}
    (peaks,activity)
}
pub fn load(job:&DownloadJob)->Result<ChatReport,String>{
    let raw=raw_path(job);let(messages,untimed,malformed)=parse(&raw)?;let(peaks,activity)=analyze(&messages);
    let count=|kind:&str|messages.iter().filter(|m|m.kind==kind).count();
    let edit=fs::read(edit_path(job)).ok().and_then(|b|serde_json::from_slice(&b).ok());
    Ok(ChatReport{title:job.request.item.title.clone(),duration:job.request.item.duration.unwrap_or_else(||messages.last().map(|m|m.time+10.0).unwrap_or(0.0)),normal:count("normal"),super_chat:count("superchat"),super_sticker:count("supersticker"),memberships:count("membership"),participants:messages.iter().map(|m|if m.author_id.is_empty(){&m.author}else{&m.author_id}).collect::<HashSet<_>>().len(),untimed,malformed,peaks,activity,raw_path:raw.to_string_lossy().into_owned(),messages,edit})
}
pub async fn download(app:&AppHandle,job:&DownloadJob)->Result<ChatReport,String>{
    let operation=Operation::begin_resource(&job.job_id,Some(format!("archive:{}",raw_path(job).to_string_lossy())))?;
    if raw_path(job).is_file(){return load(job)}
    let directory=folder(job);if !directory.is_dir(){return Err("Video klasörü bulunamadı. Önce videoyu indirin.".into())}
    status(app,job,"downloading","Sohbet tekrarı indiriliyor…",None);
    let result=async {
        let bins=BinaryPaths::resolve(app)?;
        let stage=directory.join(format!(".chat-download-{}",uuid::Uuid::new_v4()));fs::create_dir(&stage).map_err(|e|e.to_string())?;
        let mut cmd=tokio::process::Command::new(&bins.ytdlp);
        cmd.args(["--js-runtimes",&bins.deno_runtime_arg(),"--remote-components","ejs:github","--encoding","utf-8","--skip-download","--no-progress","--no-playlist","--write-subs","--sub-langs","live_chat","--no-write-auto-subs","--retries","5","--fragment-retries","10","--output"]).arg(stage.join("chat.%(ext)s")).arg(&job.request.item.webpage_url).stdin(Stdio::null()).kill_on_drop(true);
        #[cfg(windows)] {cmd.creation_flags(0x08000000);}
        let future=cmd.output();tokio::pin!(future);
        let output=loop{tokio::select!{out=&mut future=>break out.map_err(|e|e.to_string())?,_=tokio::time::sleep(std::time::Duration::from_millis(200))=>{if operation.cancel.load(Ordering::Relaxed){return Err("Sohbet indirme iptal edildi. Kısmi ham dosya korunuyor.".into())}}}};
        fs::write(stage.join("download.log"),&output.stderr).map_err(|e|e.to_string())?;
        if !output.status.success(){return Err(format!("Sohbet indirilemedi: {}",String::from_utf8_lossy(&output.stderr)))}
        let source=stage.join("chat.live_chat.json");
        if !source.is_file(){return Err("YouTube bu video için sohbet tekrarı sunmuyor (kapalı, işlenmemiş veya erişilemiyor).".into())}
        // Move the exact downloader bytes; never rewrite the raw archive.
        if !raw_path(job).exists(){fs::rename(source,raw_path(job)).map_err(|e|e.to_string())?;}
        let report=load(job)?;
        fs::write(directory.join(format!("{}.processed.json",base(job))),serde_json::to_vec_pretty(&report.messages).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        Ok(report)
    }.await;
    match &result{Ok(_)=>status(app,job,"ready","Sohbet hazır",None),Err(e)=>status(app,job,"failed",e,None)}
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn google_standard_emoji_images_are_preserved(){
        for (name,code) in [(":robot:","1f916"),(":smiling_face_with_smiling_eyes:","1f60a")]{
            let url=format!("https://fonts.gstatic.com/s/e/notoemoji/15.1/{code}/72.png");
            let runs=rich_runs(&serde_json::json!({"runs":[{"emoji":{"shortcuts":[name],"image":{"thumbnails":[{"url":url}]}}}]}));
            assert_eq!(runs.len(),1);assert_eq!(runs[0].text,name);assert_eq!(runs[0].image.as_deref(),Some(url.as_str()));
        }
        assert!(!valid_image_url("https://fonts.gstatic.com.example.org/emoji.png"));
        assert!(!valid_image_url("http://fonts.gstatic.com/emoji.png"));
    }

    use super::*;
    #[test]fn parses_replay_deduplicates_and_counts_paid_types(){
        let data=serde_json::json!({"replayChatItemAction":{"videoOffsetTimeMsec":"2500","actions":[{"addChatItemAction":{"item":{"liveChatPaidMessageRenderer":{"id":"a","authorName":{"simpleText":"İsim"},"message":{"runs":[{"text":"Merhaba "},{"emoji":{"shortcuts":[":kalp:"]}}]},"purchaseAmountText":{"simpleText":"₺50"}}}}}]}});
        let(mut out,mut seen,mut missing)=(Vec::new(),HashSet::new(),0);walk(&data,None,&mut out,&mut seen,&mut missing);walk(&data,None,&mut out,&mut seen,&mut missing);
        assert_eq!(out.len(),1);assert_eq!(out[0].time,2.5);assert_eq!(out[0].kind,"superchat");assert_eq!(out[0].text,"Merhaba :kalp:");
    }
    #[test]fn sliding_windows_are_half_open(){
        let messages=[0.0,14.9,15.0,16.0].iter().map(|t|Message{runs:vec![],id:String::new(),time:*t,author:String::new(),author_id:String::new(),text:String::new(),kind:"normal".into(),amount:None}).collect::<Vec<_>>();
        let(peaks,_)=analyze(&messages);assert_eq!(peaks[0].count,3);assert_eq!(peaks[0].start,2.0);
    }
    #[test]fn sticker_membership_and_later_timed_copy_are_retained(){
        let message=serde_json::json!({"liveChatTextMessageRenderer":{"id":"a","message":{"simpleText":"Hello"}}});
        let(mut out,mut seen,mut missing)=(Vec::new(),HashSet::new(),0);
        walk(&message,None,&mut out,&mut seen,&mut missing);
        walk(&message,Some(1.0),&mut out,&mut seen,&mut missing);
        walk(&serde_json::json!({"liveChatPaidStickerRenderer":{"id":"b","purchaseAmountText":{"simpleText":"$5.00"},"sticker":{"accessibility":{"accessibilityData":{"label":"Happy sticker"}}}}}),Some(2.0),&mut out,&mut seen,&mut missing);
        walk(&serde_json::json!({"liveChatMembershipItemRenderer":{"id":"c","headerSubtext":{"simpleText":"New member"}}}),Some(3.0),&mut out,&mut seen,&mut missing);
        assert_eq!(out.len(),3);assert_eq!(out[1].kind,"supersticker");assert_eq!(out[1].text,"Happy sticker");assert_eq!(out[2].kind,"membership");assert_eq!(missing,1);
    }
}
