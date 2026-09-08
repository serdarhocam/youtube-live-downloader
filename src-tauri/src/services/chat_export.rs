use super::{chat::{self, Message, Operation}, chat_edit::{Cut, EditMap}, binaries::BinaryPaths};
use crate::models::DownloadJob;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Stdio, sync::atomic::Ordering};
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase", default)]
pub struct WidgetOptions { pub width:u32, pub height:u32, pub font_size:u32, pub history_font_size:u32, pub history_opacity:f64, pub max_messages:usize, pub lifetime:f64, pub carry_seconds:f64, pub carry_count:usize, pub offset:f64, pub fps:f64 }
impl Default for WidgetOptions {fn default()->Self{Self{width:480,height:900,font_size:24,history_font_size:20,history_opacity:0.65,max_messages:5,lifetime:12.0,carry_seconds:8.0,carry_count:5,offset:0.0,fps:30.0}}}
impl WidgetOptions {
    pub fn validate(&self)->Result<(),String>{
        if !(12..=64).contains(&self.history_font_size)||!self.history_opacity.is_finite()||!(0.0..=1.0).contains(&self.history_opacity)||!(240..=1920).contains(&self.width)||!(240..=2160).contains(&self.height)||!(12..=64).contains(&self.font_size)||!(1..=12).contains(&self.max_messages)||self.carry_count>12 || !self.fps.is_finite() || !(1.0..=60.0).contains(&self.fps) || !self.offset.is_finite() || self.offset.abs()>86400.0 || !self.lifetime.is_finite() || !(1.0..=120.0).contains(&self.lifetime)||!self.carry_seconds.is_finite()||!(0.0..=60.0).contains(&self.carry_seconds){return Err("Widget ayarları geçersiz.".into())}Ok(())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct TimedMessage { pub message:Message, pub start:f64, pub end:f64, pub past:bool }
pub fn retime(messages:&[Message],edit:&EditMap,options:&WidgetOptions)->Vec<TimedMessage>{
    let mut result=Vec::new();let mut previous_out=0.0;
    // Merge artificial splits that retain continuous source and timeline time.
    let mut cuts:Vec<Cut>=Vec::new();
    for cut in &edit.cuts {if let Some(last)=cuts.last_mut(){if (last.end-cut.start).abs()<0.001&&(last.source_out-cut.source_in).abs()<0.001{last.end=cut.end;last.source_out=cut.source_out;continue}}cuts.push(cut.clone())}
    for cut in &cuts {
        if cut.source_in>previous_out && options.carry_seconds>0.0 {
            let low=messages.partition_point(|m|m.time+options.offset<previous_out);
            let high=messages.partition_point(|m|m.time+options.offset<cut.source_in);
            for m in &messages[low.max(high.saturating_sub(options.carry_count))..high]{result.push(TimedMessage{message:m.clone(),start:cut.start,end:(cut.start+options.carry_seconds).min(edit.duration),past:true})}
        }
        let low=messages.partition_point(|m|m.time+options.offset<cut.source_in);
        let high=messages.partition_point(|m|m.time+options.offset<cut.source_out);
        for m in &messages[low..high]{
            let start=cut.start+(m.time+options.offset-cut.source_in);
            result.push(TimedMessage{message:m.clone(),start,end:(start+options.lifetime).min(edit.duration),past:false});
        }
        previous_out=cut.source_out;
    }
    result.sort_by(|a,b|a.start.total_cmp(&b.start).then_with(||b.past.cmp(&a.past)));result
}
pub fn identity(duration:f64,fps:f64)->EditMap{EditMap{name:"Original".into(),fps,duration,cuts:vec![Cut{source_in:0.0,source_out:duration,start:0.0,end:duration}]}}
fn clock(seconds:f64,srt:bool)->String{let units=if srt{1000}else{100};let n=(seconds.max(0.0)*units as f64).round() as u64;let sec=n/units;format!("{:02}:{:02}:{:02}{}{:0width$}",sec/3600,(sec/60)%60,sec%60,if srt{','}else{'.'},n%units,width=if srt{3}else{2})}
fn label(m:&TimedMessage)->String{format!("{}{}{}","",m.message.author,m.message.amount.as_ref().map(|a|format!(" • {a}")).unwrap_or_default())}
pub fn srt(mapped:&[TimedMessage],filter:&str)->String{
    // Group simultaneous arrivals and cap each cue at the next arrival to avoid overlapping subtitles.
    let items:Vec<_>=mapped.iter().filter(|m|filter!="paid"||matches!(m.message.kind.as_str(),"superchat"|"supersticker")).collect();
    let mut result=String::new();let(mut i,mut cue)=(0,1);
    while i<items.len(){let start=items[i].start;let mut j=i+1;while j<items.len()&&(items[j].start-start).abs()<0.001{j+=1}
        let end=items[i..j].iter().map(|m|m.end).fold(0.0,f64::max).min(items.get(j).map(|m|m.start).unwrap_or(f64::INFINITY));
        if end-start>=0.001{let body=items[i..j].iter().map(|m|format!("{}: {}",label(m),m.message.text).replace(['\r','\n']," ").replace('<',"‹").replace('>',"›")).collect::<Vec<_>>().join("\n");result.push_str(&format!("{cue}\n{} --> {}\n{body}\n\n",clock(start,true),clock(end,true)));cue+=1}i=j;
    }result
}
pub fn prepare(job:&DownloadJob,options:&WidgetOptions,use_edit:bool)->Result<(Vec<TimedMessage>,EditMap),String>{
    options.validate()?;let report=chat::load(job)?;
    let edit=if use_edit{report.edit.ok_or("Önce Resolve XML içe aktarın.")?}else{identity(report.duration,options.fps)};
    if !edit.duration.is_finite()||edit.duration<=0.0{return Err("Video süresi geçersiz.".into())}
    Ok((retime(&report.messages,&edit,options),edit))
}
pub fn new_export_dir(job:&DownloadJob)->Result<PathBuf,String>{let path=chat::folder(job).join(format!("chat-export-{}-{}",chrono::Local::now().format("%Y%m%d-%H%M%S"),&uuid::Uuid::new_v4().to_string()[..8]));fs::create_dir(&path).map_err(|e|e.to_string())?;Ok(path)}
pub fn export_text(job:&DownloadJob,options:&WidgetOptions,use_edit:bool,kind:&str)->Result<String,String>{
    let(mapped,edit)=prepare(job,options,use_edit)?;let dir=new_export_dir(job)?;
    let content=match kind{"all"|"paid"=>srt(&mapped,kind),"peaks"=>{
        let mut messages:Vec<_>=mapped.iter().filter(|m|!m.past).map(|m|{let mut v=m.message.clone();v.time=m.start;v}).collect();messages.sort_by(|a,b|a.time.total_cmp(&b.time));
        let(mut peaks,_)=chat::analyze(&messages);peaks.sort_by(|a,b|a.start.total_cmp(&b.start));
        peaks.iter().enumerate().map(|(i,p)|format!("{}\n{} --> {}\n{} mesaj / 15 saniye\n\n",i+1,clock(p.start,true),clock(p.end.min(edit.duration),true),p.count)).collect()
    },_=>return Err("Unknown export type".into())};
    let path=dir.join(format!("chat-{kind}.srt"));fs::write(&path,content).map_err(|e|e.to_string())?;Ok(path.to_string_lossy().into_owned())
}
pub async fn render(app:&AppHandle,job:&DownloadJob,options:WidgetOptions,use_edit:bool,preview:bool)->Result<String,String>{
    let operation=Operation::begin(&job.job_id)?;
    let result=async{
        let(mut mapped,edit)=prepare(job,&options,use_edit)?;let dir=new_export_dir(job)?;
        fs::write(dir.join("timing.json"),serde_json::to_vec_pretty(&edit).unwrap()).map_err(|e|e.to_string())?;
        fs::write(dir.join("widget.json"),serde_json::to_vec_pretty(&options).unwrap()).map_err(|e|e.to_string())?;
        let duration=if preview{edit.duration.min(15.0)}else{edit.duration};
        mapped.retain(|m|m.start<duration);
        chat::status(app,job,"rendering","Emote görselleri ve sohbet kareleri hazırlanıyor…",None);
        let images=super::chat_raster::assets(&mapped,&chat::folder(job).join("chat-emotes"),&operation.cancel).await?;
        let render_messages=mapped.clone();let render_options=options.clone();let render_dir=dir.clone();let cancel=operation.cancel.clone();let fps=edit.fps;
        tokio::task::spawn_blocking(move||super::chat_raster::scenes(&render_messages,&render_options,&images,&render_dir,duration,fps,cancel)).await.map_err(|e|e.to_string())??;
        let bins=BinaryPaths::resolve(app)?;let log=fs::File::create(dir.join("render.log")).map_err(|e|e.to_string())?;
        let output=dir.join(if preview{"chat-preview.mov"}else{"chat-overlay.mov"});
        chat::status(app,job,"rendering","Şeffaf sohbet videosu oluşturuluyor…",None);
        let mut cmd=tokio::process::Command::new(bins.ffmpeg);
        cmd.current_dir(&dir).args(["-hide_banner","-nostdin","-n","-f","concat","-safe","0","-i","scenes.ffconcat","-vf"]).arg(format!("fps={},format=argb",edit.fps)).arg("-t").arg(duration.to_string()).args(["-an","-c:v","qtrle","-pix_fmt","argb","-progress","pipe:1","-nostats"]).arg(&output)
            .stdout(Stdio::piped()).stderr(Stdio::from(log)).stdin(Stdio::null()).kill_on_drop(true);
        #[cfg(windows)]{cmd.creation_flags(0x08000000);}
        let mut child=cmd.spawn().map_err(|e|e.to_string())?;
        let mut lines=BufReader::new(child.stdout.take().unwrap()).lines();
        loop{tokio::select!{
            line=lines.next_line()=>{match line.map_err(|e|e.to_string())?{None=>break,Some(line)=>{if let Some(time)=line.strip_prefix("out_time_us=").and_then(|v|v.parse::<f64>().ok()){chat::status(app,job,"rendering","Şeffaf sohbet videosu oluşturuluyor…",Some((time/1_000_000.0/duration*100.0).min(100.0)))}}}},
            _=tokio::time::sleep(std::time::Duration::from_millis(200))=>{if operation.cancel.load(Ordering::Relaxed){child.kill().await.map_err(|e|e.to_string())?;return Err("Dışa aktarım iptal edildi. Yarım çıktı export klasöründe kaldı.".into())}}
        }}
        if !child.wait().await.map_err(|e|e.to_string())?.success(){return Err(format!("FFmpeg dışa aktarımı başarısız. {}",fs::read_to_string(dir.join("render.log")).unwrap_or_default()))}
        // Only remove generated frame files from this newly created export directory.
        if let Ok(entries)=fs::read_dir(&dir){for entry in entries.flatten(){let name=entry.file_name().to_string_lossy().into_owned();if name.starts_with("scene-")&&name.ends_with(".png"){let _=fs::remove_file(entry.path());}}}
        let _=fs::remove_file(dir.join("scenes.ffconcat"));
        Ok(output.to_string_lossy().into_owned())
    }.await;
    match &result{Ok(path)=>chat::status(app,job,"exported",path,None),Err(e)=>chat::status(app,job,"failed",e,None)}result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires YTLD_CHAT_FIXTURE pointing to a local yt-dlp replay archive"]
    fn validate_local_archive_and_generate_render_fixture(){
        let path=PathBuf::from(std::env::var("YTLD_CHAT_FIXTURE").unwrap());
        let before=fs::read(&path).unwrap();
        let(messages,untimed,malformed)=chat::parse(&path).unwrap();
        assert!(!messages.is_empty());assert_eq!(malformed,0);
        let dir=path.parent().unwrap();
        fs::write(dir.join("normalized.json"),serde_json::to_vec_pretty(&messages).unwrap()).unwrap();
        let mut options=WidgetOptions::default();options.offset=-messages[0].time;
        let mapped=retime(&messages,&identity(15.0,30.0),&options);
        fs::write(dir.join("timed.json"),serde_json::to_vec_pretty(&mapped).unwrap()).unwrap();
        assert_eq!(before,fs::read(&path).unwrap());
        println!("Parsed {} messages, {} untimed; raw bytes unchanged",messages.len(),untimed);
    }
    fn message(time:f64)->Message{Message{runs:vec![],id:time.to_string(),time,author:"Test".into(),author_id:"x".into(),text:"Merhaba {\\pos(1,1)}".into(),kind:"normal".into(),amount:None}}
    #[test]fn cuts_keep_recent_history_and_shift_new_messages(){
        let edit=EditMap{name:"Test".into(),fps:30.0,duration:270.0,cuts:vec![Cut{source_in:0.0,source_out:120.0,start:0.0,end:120.0},Cut{source_in:150.0,source_out:300.0,start:120.0,end:270.0}]};
        let options=WidgetOptions::default();let mapped=retime(&[message(125.0),message(140.0),message(160.0)],&edit,&options);
        assert!(mapped[0].past);assert_eq!(mapped[0].start,120.0);assert_eq!(mapped[0].end,128.0);assert_eq!(mapped[2].start,130.0);assert!(!mapped[2].past);
        assert!(srt(&mapped,"all").contains("00:02:10,000"));
    }
    #[test]fn configured_lifetimes_survive_short_cuts_and_srt_uses_edited_time(){
        let edit=EditMap{name:"Cuts".into(),fps:30.0,duration:30.0,cuts:vec![Cut{source_in:0.0,source_out:2.0,start:0.0,end:2.0},Cut{source_in:10.0,source_out:38.0,start:2.0,end:30.0}]};
        let mut o=WidgetOptions::default();o.lifetime=12.0;o.carry_seconds=8.0;
        let mapped=retime(&[message(1.0),message(5.0),message(11.0)],&edit,&o);
        assert_eq!((mapped[0].start,mapped[0].end),(1.0,13.0));
        assert_eq!((mapped[1].start,mapped[1].end,mapped[1].past),(2.0,10.0,true));
        assert_eq!((mapped[2].start,mapped[2].end),(3.0,15.0));
        let subtitles=srt(&mapped,"all");assert!(subtitles.contains("00:00:03,000"));assert!(!subtitles.contains("[Önceki]"));
    }
    #[test]fn boundary_and_empty_cut_handling(){
        let edit=identity(10.0,30.0);let mapped=retime(&[message(0.0),message(10.0)],&edit,&WidgetOptions::default());assert_eq!(mapped.len(),1);assert_eq!(mapped[0].end,10.0);
    }
    #[test]fn history_is_bounded_and_contiguous_splits_do_not_clear_messages(){
        let mut o=WidgetOptions::default();o.carry_count=2;
        let edit=EditMap{name:"Test".into(),fps:30.0,duration:20.0,cuts:vec![Cut{source_in:10.0,source_out:20.0,start:0.0,end:10.0},Cut{source_in:20.0,source_out:30.0,start:10.0,end:20.0}]};
        let mapped=retime(&[message(1.0),message(2.0),message(3.0),message(19.0)],&edit,&o);
        assert_eq!(mapped.len(),3);assert_eq!(mapped[0].message.time,2.0);assert_eq!(mapped[2].end,20.0);
        o.carry_count=0;assert_eq!(retime(&[message(1.0)],&edit,&o).len(),0);
    }
}
