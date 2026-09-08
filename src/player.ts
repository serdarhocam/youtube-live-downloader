import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export async function initPlayer(app:HTMLElement){
  const query=new URLSearchParams(location.search),tr=query.get("lang")==="tr";
  document.body.classList.add("player-page");
  app.innerHTML=`<main class="player"><video controls playsinline preload="metadata"></video><div class="player-controls"><button id="play">${tr?"Başlat":"Play"}</button><button id="pause">${tr?"Duraklat":"Pause"}</button><button id="stop">${tr?"Durdur":"Stop"}</button><button id="back">−10 ${tr?"saniye":"seconds"}</button><button id="forward">+30 ${tr?"saniye":"seconds"}</button><label>${tr?"Hız":"Speed"} <select id="speed">${[.25,.5,.75,1,1.25,1.5,2,3,4,5,6,8,10].map(n=>`<option value="${n}" ${n===1?"selected":""}>${n}×</option>`).join("")}</select></label></div><p id="player-error" role="alert"></p></main>`;
  const video=app.querySelector("video")!,error=app.querySelector<HTMLElement>("#player-error")!;
  const report=(e:unknown)=>error.textContent=String(e);
  let requested=Number(query.get("seconds")||0);
  const go=(seconds:number)=>{if(Number.isFinite(seconds)&&seconds>=0){requested=seconds;if(Number.isFinite(video.duration))video.currentTime=Math.min(seconds,video.duration)}};
  video.addEventListener("loadedmetadata",()=>go(requested));
  await listen<number>("player-seek",e=>go(e.payload));
  const seek=(delta:number)=>{if(Number.isFinite(video.duration))video.currentTime=Math.max(0,Math.min(video.duration,video.currentTime+delta))};
  app.querySelector("#play")!.addEventListener("click",()=>video.play().catch(report));
  app.querySelector("#pause")!.addEventListener("click",()=>video.pause());
  app.querySelector("#stop")!.addEventListener("click",()=>{video.pause();if(video.readyState)video.currentTime=0});
  app.querySelector("#back")!.addEventListener("click",()=>seek(-10));
  app.querySelector("#forward")!.addEventListener("click",()=>seek(30));
  app.querySelector("#speed")!.addEventListener("change",e=>{try{video.playbackRate=Number((e.target as HTMLSelectElement).value)}catch(e){report(e)}});
  video.addEventListener("error",()=>report(tr?"Video oynatılamadı. Dosya silinmiş veya video/ses biçimi bu bilgisayarda desteklenmiyor olabilir.":"Cannot play video. The file may be missing or its video/audio codec may not be supported on this computer."));
  try{
    const media=await invoke<{videoPath:string}>("get_playback_media",{jobId:query.get("player")});
    video.src=convertFileSrc(media.videoPath);
  }catch(e){report(e)}
}
