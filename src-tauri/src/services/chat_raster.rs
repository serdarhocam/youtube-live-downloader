//! Rasterize the same inline text/emote flow as the widget, with transparent pixels.
use super::{chat, chat_export::{TimedMessage,WidgetOptions}};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use image::{Rgba,RgbaImage,imageops};
use sha2::{Digest,Sha256};
use std::{collections::HashMap,fs,path::Path,sync::{Arc,atomic::{AtomicBool,Ordering}}};

pub async fn assets(mapped:&[TimedMessage],cache:&Path,cancel:&AtomicBool)->Result<HashMap<String,RgbaImage>,String>{
    fs::create_dir_all(cache).map_err(|e|e.to_string())?;
    let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(20)).build().map_err(|e|e.to_string())?;
    let mut images=HashMap::new();
    for run in mapped.iter().flat_map(|m|&m.message.runs){
        let Some(url)=&run.image else{continue};if images.contains_key(url){continue}
        if cancel.load(Ordering::Relaxed){return Err("Dışa aktarım iptal edildi.".into())}
        if !chat::valid_image_url(url){return Err("Emote görsel adresi geçersiz.".into())}
        let path=cache.join(format!("{:x}.png",Sha256::digest(url.as_bytes())));
        let cached=fs::read(&path).ok().and_then(|bytes|decode(&bytes).ok());
        let img=if let Some(img)=cached{img}else{
            let fetch=async{
                let mut response=client.get(url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
                let mut bytes=Vec::new();while let Some(chunk)=response.chunk().await.map_err(|e|e.to_string())?{if bytes.len()+chunk.len()>4*1024*1024{return Err("Emote görseli çok büyük.".to_string())}bytes.extend_from_slice(&chunk)}
                decode(&bytes)
            };
            let img=tokio::select!{result=fetch=>result.map_err(|e|format!("Emote indirilemedi ({}): {e}. İnternet bağlantısını kontrol edip yeniden deneyin.",run.text))?,_=async{loop{tokio::time::sleep(std::time::Duration::from_millis(100)).await;if cancel.load(Ordering::Relaxed){break}}}=>return Err("Dışa aktarım iptal edildi.".into())};
            img.save(&path).map_err(|e|e.to_string())?;img
        };images.insert(url.clone(),img);
    }Ok(images)
}
fn decode(bytes:&[u8])->Result<RgbaImage,String>{
    let mut reader=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e|e.to_string())?;
    let mut limits=image::Limits::default();limits.max_image_width=Some(2048);limits.max_image_height=Some(2048);limits.max_alloc=Some(32*1024*1024);reader.limits(limits);
    reader.decode().map(|v|v.to_rgba8()).map_err(|e|e.to_string())
}
struct Fonts{regular:FontArc,bold:FontArc}
impl Fonts{fn load()->Result<Self,String>{
    let dir=std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(||"C:\\Windows".into())).join("Fonts");
    let read=|name:&str|FontArc::try_from_vec(fs::read(dir.join(name)).map_err(|e|format!("Segoe UI yazı tipi okunamadı: {e}"))?).map_err(|e|e.to_string());
    Ok(Self{regular:read("segoeui.ttf")?,bold:read("segoeuib.ttf")?})
}}
#[derive(Clone)]
enum Part{Text(String,bool,[u8;3]),Emote(String)}
fn parts(m:&TimedMessage)->Vec<Part>{
    let colors=[[114,223,155],[123,193,242],[203,159,228],[242,220,116]];
    let hash=m.message.author.bytes().fold(0usize,|a,b|a.wrapping_mul(31).wrapping_add(b as usize));
    let color=if m.message.kind=="normal"{colors[hash%4]}else{[255,217,91]};
    let mut result=vec![Part::Text(format!("{}{}{} ","",m.message.author,m.message.amount.as_ref().map(|v|format!(" {v}")).unwrap_or_default()),true,color)];
    if m.message.runs.is_empty(){result.push(Part::Text(m.message.text.clone(),false,[255;3]))}
    else{for r in &m.message.runs{result.push(if let Some(url)=&r.image{Part::Emote(url.clone())}else{Part::Text(r.text.clone(),false,[255;3])})}}
    result
}
fn width(text:&str,font:&FontArc,size:f32)->f32{let size=size*font.height_unscaled()/font.units_per_em().unwrap_or(2048.0);let scaled=font.as_scaled(PxScale::from(size));text.chars().map(|c|scaled.h_advance(font.glyph_id(c))).sum()}
fn glyph(img:&mut RgbaImage,font:&FontArc,size:f32,c:char,x:f32,y:f32,color:[u8;3]){
    let size=size*font.height_unscaled()/font.units_per_em().unwrap_or(2048.0);let scaled=font.as_scaled(PxScale::from(size));let g=font.glyph_id(c).with_scale_and_position(size,point(x,y+scaled.ascent()));
    if let Some(outline)=font.outline_glyph(g){let bounds=outline.px_bounds();outline.draw(|gx,gy,coverage|{
        let px=bounds.min.x as i32+gx as i32;let py=bounds.min.y as i32+gy as i32;
        if px>=0&&py>=0&&px<img.width() as i32&&py<img.height() as i32{use image::Pixel;img.get_pixel_mut(px as u32,py as u32).blend(&Rgba([color[0],color[1],color[2],(coverage*255.0).round() as u8]));}
    })}
}
fn tile(m:&TimedMessage,o:&WidgetOptions,fonts:&Fonts,images:&HashMap<String,RgbaImage>)->RgbaImage{
    let mut styled=o.clone();if m.past{styled.font_size=o.history_font_size;}let o=&styled;
    let w=o.width-40;let line=(o.font_size as f32*1.35).ceil();let size=o.font_size as f32;
    let mut foreground=RgbaImage::new(w,o.height);let(mut x,mut y)=(4.0,4.0);let limit=w as f32-4.0;
    for part in parts(m){match part{
        Part::Text(text,bold,color)=>{
            let font=if bold{&fonts.bold}else{&fonts.regular};
            // Keep words together, breaking an oversized word only at the width limit.
            for word in text.split_inclusive(char::is_whitespace){
                let word_width=width(word.trim_end(),font,size);
                if x>4.0&&x+word_width>limit{x=4.0;y+=line}
                for c in word.chars(){if c=='\n'{x=4.0;y+=line;continue}if c=='\r'{continue}
                    let advance=width(&c.to_string(),font,size);
                    if x+advance>limit{x=4.0;y+=line;if c.is_whitespace(){continue}}
                    glyph(&mut foreground,font,size,c,x,y,color);x+=advance;
                }
            }
        },Part::Emote(url)=>{let edge=(size*1.25).round() as u32;if x+edge as f32>limit{x=4.0;y+=line}
            if let Some(img)=images.get(&url){let emote=imageops::thumbnail(img,edge,edge);imageops::overlay(&mut foreground,&emote,x.round() as i64,y.round() as i64)}x+=edge as f32;
        }
    }}
    let height=((y+line+5.0).ceil() as u32).min(o.height);
    let foreground=imageops::crop_imm(&foreground,0,0,w,height).to_image();
    let mut outline=RgbaImage::new(w,height);
    for (x,y,p) in foreground.enumerate_pixels(){if p[3]==0{continue}for dy in -1i32..=1{for dx in -1i32..=1{let(px,py)=(x as i32+dx,y as i32+dy);if px>=0&&py>=0&&px<w as i32&&py<height as i32{let q=outline.get_pixel_mut(px as u32,py as u32);q[3]=q[3].max(p[3]);}}}}
    let shadow=imageops::blur(&outline,1.6);let mut output=RgbaImage::new(w,height);imageops::overlay(&mut output,&shadow,0,1);imageops::overlay(&mut output,&outline,0,0);imageops::overlay(&mut output,&foreground,0,0);if m.past{for pixel in output.pixels_mut(){pixel[3]=(pixel[3] as f64*o.history_opacity).round() as u8;}}output
}
/// One PNG per change, held until the next frame-aligned change. No full-duration frame dump.
pub fn scenes(mapped:&[TimedMessage],o:&WidgetOptions,images:&HashMap<String,RgbaImage>,dir:&Path,duration:f64,fps:f64,cancel:Arc<AtomicBool>)->Result<(),String>{
    let fonts=Fonts::load()?;let frames=(duration*fps).ceil() as u64;
    let frame=|t:f64|(t*fps).ceil().max(0.0) as u64;
    let mut boundaries=vec![0,frames];boundaries.extend(mapped.iter().flat_map(|m|[frame(m.start).min(frames),frame(m.end).min(frames)]));boundaries.sort_unstable();boundaries.dedup();
    let mut tiles=HashMap::new();let mut next=0;let mut active:Vec<usize>=Vec::new();let mut concat=String::new();let mut last=String::new();
    for (index,pair) in boundaries.windows(2).enumerate(){
        if cancel.load(Ordering::Relaxed){return Err("Dışa aktarım iptal edildi.".into())}
        active.retain(|i|frame(mapped[*i].end)>pair[0]);
        while next<mapped.len()&&frame(mapped[next].start)<=pair[0]{if frame(mapped[next].end)>pair[0]{active.push(next)}next+=1}
        if active.len()>o.max_messages{active.drain(..active.len()-o.max_messages);}
        tiles.retain(|i,_|active.contains(i));
        let mut scene=RgbaImage::new(o.width,o.height);let mut y=o.height as i64-20;
        for &i in active.iter().rev(){let tile=tiles.entry(i).or_insert_with(||tile(&mapped[i],o,&fonts,images));y-=tile.height() as i64;imageops::overlay(&mut scene,tile,20,y);y-=12;if y<20{break}}
        last=format!("scene-{index:06}.png");scene.save(dir.join(&last)).map_err(|e|e.to_string())?;
        concat.push_str(&format!("file '{last}'\noption framerate {fps}\nduration {:.9}\n",(pair[1]-pair[0]) as f64/fps));
    }
    concat.push_str(&format!("file '{last}'\noption framerate {fps}\n"));fs::write(dir.join("scenes.ffconcat"),concat).map_err(|e|e.to_string())
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]
    #[ignore = "Requires local chat fixture and network for YouTube emote images"]
    fn real_archive_emote_render(){
        let path=std::path::PathBuf::from(std::env::var("YTLD_CHAT_FIXTURE").unwrap());let before=fs::read(&path).unwrap();
        let (messages,_,_)=chat::parse(&path).unwrap();
        let message=messages.iter().find(|m|m.runs.iter().any(|r|r.image.is_some())).unwrap().clone();
        let mapped=vec![TimedMessage{message,start:0.0,end:3.0,past:false}];let dir=path.parent().unwrap().join("emote-render");fs::create_dir_all(&dir).unwrap();
        let cancel=Arc::new(AtomicBool::new(false));
        let images=tokio::runtime::Runtime::new().unwrap().block_on(assets(&mapped,&dir.join("cache"),&cancel)).unwrap();assert!(!images.is_empty());
        scenes(&mapped,&WidgetOptions::default(),&images,&dir,3.0,30.0,cancel).unwrap();
        fs::write(path.parent().unwrap().join("normalized.json"),serde_json::to_vec_pretty(&messages).unwrap()).unwrap();
        fs::write(path.parent().unwrap().join("timed.json"),serde_json::to_vec_pretty(&mapped).unwrap()).unwrap();
        assert_eq!(before,fs::read(&path).unwrap());
    }
    #[test]fn scene_durations_match_lifetime_at_frame_boundaries(){
        let dir=std::env::temp_dir().join(format!("ytld-timing-{}",uuid::Uuid::new_v4()));fs::create_dir(&dir).unwrap();
        let m=TimedMessage{message:chat::Message{id:"1".into(),time:0.5,author:"Test".into(),author_id:"a".into(),text:"Two seconds".into(),kind:"normal".into(),amount:None,runs:vec![]},start:0.5,end:2.5,past:false};
        scenes(&[m],&WidgetOptions::default(),&HashMap::new(),&dir,4.0,30.0,Arc::new(AtomicBool::new(false))).unwrap();
        let manifest=fs::read_to_string(dir.join("scenes.ffconcat")).unwrap();
        let durations:Vec<f64>=manifest.lines().filter_map(|line|line.strip_prefix("duration ").map(|v|v.parse().unwrap())).collect();
        assert_eq!(durations,vec![0.5,2.0,1.5]);
        for (i,visible) in [(0,false),(1,true),(2,false)]{let img=image::open(dir.join(format!("scene-{i:06}.png"))).unwrap().to_rgba8();assert_eq!(img.pixels().any(|p|p[3]>0),visible);}
        for entry in fs::read_dir(&dir).unwrap(){fs::remove_file(entry.unwrap().path()).unwrap();}fs::remove_dir(dir).unwrap();
    }
    #[test]fn emotes_and_text_have_transparent_background(){
        let m=TimedMessage{message:chat::Message{id:"1".into(),time:0.0,author:"İsim".into(),author_id:"a".into(),text:"hello".into(),kind:"normal".into(),amount:None,runs:vec![chat::MessageRun{text:":test:".into(),image:Some("fixture".into())}]},start:0.0,end:2.0,past:false};
        let mut images=HashMap::new();images.insert("fixture".into(),RgbaImage::from_pixel(20,20,Rgba([255,0,0,255])));
        let image=tile(&m,&WidgetOptions::default(),&Fonts::load().unwrap(),&images);
        assert!(image.height()<60,"nickname and emote should share one line");
        assert_eq!(image.get_pixel(image.width()-1,0)[3],0);
        assert!(image.pixels().any(|p|p.0==[255,0,0,255]));
        assert!(image.pixels().any(|p|p[0]==0&&p[3]>0));
        let mut history=m.clone();history.past=true;let mut options=WidgetOptions::default();options.history_font_size=12;options.history_opacity=0.3;
        let faded=tile(&history,&options,&Fonts::load().unwrap(),&images);
        assert!(faded.height()<image.height());assert!(faded.pixels().all(|p|p[3]<=77));
        let mut long=m.clone();long.message.runs.clear();long.message.text="Uzun mesaj genişliği kullanarak alt satıra geçmeli. ".repeat(8);
        let wrapped=tile(&long,&WidgetOptions::default(),&Fonts::load().unwrap(),&images);
        assert!(wrapped.height()>4*24,"long messages must not be clamped to three lines");
    }
}
