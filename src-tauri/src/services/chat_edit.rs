use serde::{Deserialize, Serialize};
use roxmltree::{Document, Node};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cut { pub source_in: f64, pub source_out: f64, pub start: f64, pub end: f64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditMap { pub name: String, pub fps: f64, pub duration: f64, pub cuts: Vec<Cut> }
fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> { node.children().find(|n| n.has_tag_name(name)) }
fn text<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> { child(node, name)?.text() }
fn number(node: Node, name: &str) -> Result<f64, String> { text(node, name).and_then(|v| v.parse::<f64>().ok()).filter(|n| n.is_finite()).ok_or_else(|| format!("XML: missing/invalid {name}")) }
fn rate(node: Node) -> Result<f64, String> {
    let r = child(node, "rate").ok_or("XML: missing frame rate")?;
    let fps = number(r, "timebase")?;
    if fps <= 0.0 || fps > 240.0 { return Err("XML: invalid frame rate".into()) }
    Ok(if text(r, "ntsc") == Some("TRUE") { fps * 1000.0 / 1001.0 } else { fps })
}

pub fn parse_xml(xml: &str, source_path: &str, source_duration: f64) -> Result<EditMap, String> {
    let cleaned=xml.replace("<!DOCTYPE xmeml>", "");
    let doc = Document::parse(&cleaned).map_err(|e| format!("XML okunamadı / Invalid XML: {e}"))?;
    if !doc.root_element().has_tag_name("xmeml") { return Err("Final Cut Pro 7 XML seçin; FCPXML/DRT desteklenmiyor.".into()) }
    let sequence = doc.root_element().children().find(|n| n.has_tag_name("sequence")).ok_or("XML: timeline sequence missing")?;
    let fps = rate(sequence)?;
    let video = child(sequence, "media").and_then(|n| child(n, "video")).ok_or("XML: video track missing")?;
    let tracks: Vec<_> = video.children().filter(|n| n.has_tag_name("track") && text(*n,"enabled") != Some("FALSE") && n.children().any(|c| c.has_tag_name("clipitem"))).collect();
    if tracks.len() != 1 { return Err("İlk sürüm tek etkin video kanalı ister. Yayın kesimlerini ayrı bir timeline olarak dışa aktarın.".into()) }
    if tracks[0].children().any(|n| n.has_tag_name("transitionitem")) { return Err("Geçişler desteklenmiyor; düz kesimler kullanın.".into()) }
    let expected = source_path.replace('\\', "/").to_lowercase();
    let expected_name = expected.rsplit('/').next().unwrap_or("");
    let mut cuts = Vec::new();
    for clip in tracks[0].children().filter(|n| n.has_tag_name("clipitem") && text(*n,"enabled") != Some("FALSE")) {
        if clip.descendants().any(|n| n.has_tag_name("sequence") || ((n.has_tag_name("effectid") || (n.has_tag_name("name") && n.parent().is_some_and(|p|p.has_tag_name("effect")))) && ["timeremap","time remap","timewarp","speed","reverse"].iter().any(|key|n.text().unwrap_or("").to_ascii_lowercase().contains(key)))) {
            return Err("İç içe timeline veya hız değişimi bu aktarımda desteklenmiyor. Sadece kesimleri içeren bir timeline kopyası kullanın.".into())
        }
        let file = child(clip, "file").ok_or("XML: source file missing")?;
        let file = if child(file, "pathurl").is_some() { file } else {
            doc.descendants().find(|n| n.has_tag_name("file") && n.attribute("id") == file.attribute("id") && child(*n, "pathurl").is_some()).ok_or("XML: unresolved file reference")?
        };
        let path = url::Url::parse(text(file, "pathurl").ok_or("XML: source path missing")?).ok().and_then(|u| u.to_file_path().ok()).ok_or("XML: invalid local source URL")?;
        let actual = path.to_string_lossy().replace('\\', "/").to_lowercase();
        if actual != expected && actual.rsplit('/').next() != Some(expected_name) { return Err("XML farklı bir video içeriyor; bu kaydın orijinal videosunu kullanın.".into()) }
        let source_fps = rate(clip).or_else(|_| rate(file))?;
        let cut = Cut { source_in: number(clip,"in")? / source_fps, source_out: number(clip,"out")? / source_fps, start: number(clip,"start")? / fps, end: number(clip,"end")? / fps };
        if cut.start < 0.0 || cut.source_in < 0.0 || cut.end <= cut.start || cut.source_out <= cut.source_in || cut.source_out > source_duration + 1.0 {
            return Err("XML: invalid source/timeline interval".into())
        }
        if ((cut.end-cut.start)-(cut.source_out-cut.source_in)).abs() > 1.5 / fps { return Err("Hız değiştirilmiş klipler desteklenmiyor.".into()) }
        cuts.push(cut);
    }
    cuts.sort_by(|a,b| a.start.total_cmp(&b.start));
    if cuts.is_empty() || cuts.windows(2).any(|w| w[1].start < w[0].end - 0.001 || w[1].source_in < w[0].source_out - 0.001) {
        return Err("Kesimler kaynak sırasını korumalı; çakışma, tekrar veya ters sıralama desteklenmiyor.".into())
    }
    let duration = (number(sequence,"duration")? / fps).max(cuts.last().unwrap().end);
    if duration > 7.0*86400.0 { return Err("Timeline is too long".into()) }
    Ok(EditMap { name: text(sequence,"name").unwrap_or("Resolve").into(), fps, duration, cuts })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn xml(extra: &str) -> String { format!(r#"<xmeml><sequence><name>Edit</name><rate><timebase>30</timebase><ntsc>FALSE</ntsc></rate><duration>8100</duration><media><video><track><clipitem><rate><timebase>30</timebase></rate><start>0</start><end>3600</end><in>0</in><out>3600</out><file id="f"><pathurl>file:///C:/video.mp4</pathurl></file></clipitem><clipitem><rate><timebase>30</timebase></rate><start>3600</start><end>8100</end><in>4500</in><out>9000</out><file id="f"/>{extra}</clipitem></track></video></media></sequence></xmeml>"#) }
    #[test] fn maps_source_to_timeline_and_rejects_retime() {
        assert!(parse_xml(&xml("").replace("<clipitem>","<clipitem><name>Need for Speed</name>"),"C:/video.mp4",300.0).is_ok());
        assert!(parse_xml(&format!("<!DOCTYPE xmeml>{}",xml("<filter><effect><effectid>basic</effectid></effect></filter>")),"C:/video.mp4",300.0).is_ok());
        let map = parse_xml(&xml(""), "C:/video.mp4", 300.0).unwrap();
        assert_eq!(map.cuts[1].source_in,150.0); assert_eq!(map.cuts[1].start,120.0);
        assert!(parse_xml(&xml("<filter><effect><effectid>timeremap</effectid></effect></filter>"),"C:/video.mp4",300.0).is_err());
        assert!(parse_xml(&xml(""),"C:/other.mp4",300.0).is_err());
        assert!(parse_xml(&xml("").replace("<start>3600</start>","<start>3500</start>"),"C:/video.mp4",300.0).is_err());
    }
    #[test] fn handles_ntsc_and_rejects_reordered_or_multiple_tracks(){
        let source=xml("").replace("<ntsc>FALSE</ntsc>","<ntsc>TRUE</ntsc>").replace("<rate><timebase>30</timebase></rate>","<rate><timebase>30</timebase><ntsc>TRUE</ntsc></rate>");
        let map=parse_xml(&source,"C:/video.mp4",301.0).unwrap();assert!((map.fps-29.97002997).abs()<0.00001);
        assert!(parse_xml(&xml("").replace("<in>4500</in>","<in>3000</in>"),"C:/video.mp4",300.0).is_err());
        assert!(parse_xml(&xml("").replace("</video>","<track><clipitem/></track></video>"),"C:/video.mp4",300.0).is_err());
    }
}
