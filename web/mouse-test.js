'use strict';
const pad=document.getElementById('mousePad'), results=document.getElementById('mouseResults');
let counts={},held=new Set();
const name=n=>['Left','Middle','Right','Back','Forward'][n]||String(n);
function paint(){results.textContent=JSON.stringify({counts,held:[...held]},null,2);}
pad.addEventListener('mousedown',e=>{e.preventDefault();held.add(name(e.button));const key=name(e.button)+' down';counts[key]=(counts[key]||0)+1;paint();});
window.addEventListener('mouseup',e=>{if(!held.has(name(e.button)))return;e.preventDefault();held.delete(name(e.button));const key=name(e.button)+' up';counts[key]=(counts[key]||0)+1;paint();});
pad.addEventListener('wheel',e=>{e.preventDefault();for(const [axis,value] of [['vertical',e.deltaY],['horizontal',e.deltaX]])if(value){const key=axis+(value>0?' positive':' negative');counts[key]=(counts[key]||0)+1;}paint();},{passive:false});
for(const event of ['contextmenu','auxclick','click','dragstart'])pad.addEventListener(event,e=>e.preventDefault());
document.getElementById('resetMouse').addEventListener('click',()=>{counts={};held.clear();paint();});

const keyResults=document.getElementById('keyboardResults');
if(keyResults){
 let down=0,up=0,repeats=0,held=false;
 const paintKeys=()=>{keyResults.textContent=JSON.stringify({down,up,repeats,held},null,2);};
 window.addEventListener('keydown',e=>{if(e.code!=='KeyB')return;e.preventDefault();if(e.repeat)repeats++;else down++;held=true;paintKeys();});
 window.addEventListener('keyup',e=>{if(e.code!=='KeyB')return;e.preventDefault();up++;held=false;paintKeys();});
}
